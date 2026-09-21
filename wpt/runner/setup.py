from setuptools import setup

setup(
    name="rarog-wptrunner-r6",
    version="0.0.0",
    py_modules=["rarog_wptrunner", "rarog_wpt_helpers"],
    entry_points={
        "wptrunner.products": [
            "rarog = rarog_wptrunner:get_product",
        ],
    },
)
