from pathlib import Path

p = Path('.github/r5_windows_accessibility_harden.py')
s = p.read_text()
s = s.replace(
    '"        view: View,\\n        window: Option<Arc<Window>>,\n",',
    '"        view: View,\\n        window: Option<Arc<Window>>,\\n",',
)
s = s.replace(
    '"        view: View,\\n        platform: Arc<WindowsPlatformHost>,\\n        window: Option<Arc<Window>>,\n",',
    '"        view: View,\\n        platform: Arc<WindowsPlatformHost>,\\n        window: Option<Arc<Window>>,\\n",',
)
p.write_text(s)
