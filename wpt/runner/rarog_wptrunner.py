from __future__ import annotations

import base64
import io
import subprocess
import tempfile
from pathlib import Path

from PIL import Image
from wptrunner.browsers.base import ExecutorBrowser, NullBrowser
from wptrunner.executors import executor_kwargs as base_executor_kwargs
from wptrunner.executors.base import (
    RefTestExecutor,
    RefTestImplementation,
    TestharnessExecutor,
    reftest_result_converter,
)
from wptrunner.executors.protocol import ConnectionlessProtocol
from wptrunner.products import Product
from wptrunner.wptcommandline import require_arg

from rarog_wpt_helpers import (
    AdapterError,
    build_render_command,
    classify_render_result,
    load_allowed_paths,
    parse_viewport,
    selected_url_to_path,
    unsupported_testharness_message,
)


class RarogBrowser(NullBrowser):
    def __init__(
        self,
        logger,
        *,
        binary,
        rarog_wpt_root,
        rarog_selection,
        manager_number,
        **kwargs,
    ):
        super().__init__(logger, manager_number=manager_number, **kwargs)
        self.binary = binary
        self.rarog_wpt_root = rarog_wpt_root
        self.rarog_selection = rarog_selection

    def executor_browser(self):
        return ExecutorBrowser, {
            "binary": self.binary,
            "rarog_wpt_root": self.rarog_wpt_root,
            "rarog_selection": self.rarog_selection,
        }


class _RarogExecutorMixin:
    def __init__(self, logger, browser, server_config, **kwargs):
        super().__init__(logger, browser, server_config, **kwargs)
        self.binary = Path(browser.binary).resolve()
        self.wpt_root = Path(browser.rarog_wpt_root).resolve()
        self.selection = Path(browser.rarog_selection).resolve()
        self.allowed_paths = load_allowed_paths(self.selection)
        self.protocol = ConnectionlessProtocol(self, browser)

    def setup(self, runner, protocol=None):
        self.runner = runner
        self.runner.send_message("init_succeeded")
        return True

    def _selected_path(self, test_url):
        relative = selected_url_to_path(test_url, self.allowed_paths)
        source = (self.wpt_root / Path(*relative.split("/"))).resolve()
        try:
            source.relative_to(self.wpt_root)
        except ValueError as error:
            raise AdapterError(
                f"selected path escapes WPT checkout: {relative}"
            ) from error
        if not source.is_file():
            raise AdapterError(f"selected WPT file is missing: {relative}")
        return source


class RarogUnsupportedTestharnessExecutor(_RarogExecutorMixin, TestharnessExecutor):
    def do_test(self, test):
        self._selected_path(test.url)
        return (
            test.make_result(
                "ERROR",
                unsupported_testharness_message(test.url),
            ),
            [],
        )


class RarogRefTestExecutor(_RarogExecutorMixin, RefTestExecutor):
    convert_result = reftest_result_converter

    def __init__(self, logger, browser, server_config, **kwargs):
        super().__init__(logger, browser, server_config, **kwargs)
        self.implementation = RefTestImplementation(self)

    def reset(self):
        self.implementation.reset()

    def do_test(self, test):
        self.test = test
        return self.convert_result(test, self.implementation.run_test(test))

    def screenshot(self, test, viewport_size, dpi, page_ranges):
        if dpi not in (None, 96):
            return False, (
                "INTERNAL-ERROR",
                f"R6 Rarog renderer does not support non-default dpi {dpi!r}",
            )
        if page_ranges not in (None, []):
            return False, (
                "INTERNAL-ERROR",
                "R6 Rarog renderer does not support print page ranges",
            )

        try:
            source = self._selected_path(test.url)
            viewport = parse_viewport(viewport_size)
        except AdapterError as error:
            return False, ("INTERNAL-ERROR", str(error))

        with tempfile.TemporaryDirectory(prefix="rarog-wpt-") as directory:
            ppm_path = Path(directory) / "frame.ppm"
            command = build_render_command(
                self.binary,
                source,
                ppm_path,
                viewport,
            )
            timed_out = False
            diagnostic = ""
            returncode = None
            try:
                completed = subprocess.run(
                    command,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                    timeout=self.test.timeout * self.timeout_multiplier + 5,
                    check=False,
                )
                returncode = completed.returncode
                diagnostic = completed.stderr
            except subprocess.TimeoutExpired as error:
                timed_out = True
                diagnostic = (error.stderr or "") if isinstance(error.stderr, str) else ""

            failure = classify_render_result(
                returncode=returncode,
                timed_out=timed_out,
                output_exists=ppm_path.is_file(),
                diagnostic=diagnostic,
            )
            if failure is not None:
                return False, failure

            try:
                with Image.open(ppm_path) as image:
                    output = io.BytesIO()
                    image.convert("RGB").save(output, format="PNG")
            except (OSError, ValueError) as error:
                return False, (
                    "INTERNAL-ERROR",
                    f"cannot convert Rarog PPM screenshot to PNG: {error}",
                )

            return True, [base64.b64encode(output.getvalue()).decode("ascii")]


def check_args(**kwargs):
    require_arg(kwargs, "binary")
    require_arg(kwargs, "rarog_wpt_root")
    require_arg(kwargs, "rarog_selection")


def browser_kwargs(logger, test_type, run_info_data, config, subsuite, **kwargs):
    return {
        "binary": kwargs["binary"],
        "rarog_wpt_root": kwargs["rarog_wpt_root"],
        "rarog_selection": kwargs["rarog_selection"],
    }


def executor_kwargs(
    logger,
    test_type,
    test_environment,
    run_info_data,
    subsuite,
    **kwargs,
):
    return base_executor_kwargs(
        test_type,
        test_environment,
        run_info_data,
        subsuite,
        **kwargs,
    )


def env_extras(**kwargs):
    return []


def timeout_multiplier(*args, **kwargs):
    return 1.0


def add_arguments(parser):
    parser.add_argument(
        "--rarog-wpt-root",
        help="Exact pinned upstream WPT checkout root used by the R6 adapter",
    )
    parser.add_argument(
        "--rarog-selection",
        help="R6 file-level selection manifest used as the adapter allowlist",
    )


def run_info_extras(logger, **kwargs):
    return {"rarog_adapter": "connectionless-r6"}


def get_product():
    return Product(
        name="rarog",
        browser_classes={None: RarogBrowser},
        check_args=check_args,
        get_browser_kwargs=browser_kwargs,
        get_executor_kwargs=executor_kwargs,
        env_options={"server_host": "127.0.0.1", "bind_address": False},
        get_env_extras=env_extras,
        get_timeout_multiplier=timeout_multiplier,
        executor_classes={
            "testharness": RarogUnsupportedTestharnessExecutor,
            "reftest": RarogRefTestExecutor,
        },
        run_info_extras=run_info_extras,
        add_arguments=add_arguments,
    )
