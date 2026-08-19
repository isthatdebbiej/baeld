import importlib.util
import inspect
import os
import pathlib
import tempfile
import unittest

os.environ.setdefault(
    "BROWSER_USE_CONFIG_DIR",
    str(pathlib.Path(tempfile.gettempdir()) / "baeld-browser-use-test-config"),
)

from browser_use import BrowserSession


DRIVER_PATH = pathlib.Path(__file__).parents[1] / "workloads" / "driver" / "browser_use_driver.py"
SPEC = importlib.util.spec_from_file_location("baeld_browser_use_driver", DRIVER_PATH)
assert SPEC is not None and SPEC.loader is not None
DRIVER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DRIVER)


class BrowserUseDriverTests(unittest.TestCase):
    def test_sequence_gaps(self):
        self.assertEqual(DRIVER.sequence_gaps([1, 2, 5, 6, 9]), 4)
        self.assertEqual(DRIVER.sequence_gaps([]), 0)

    def test_browser_session_supports_owned_cdp_attachment(self):
        parameters = inspect.signature(BrowserSession).parameters
        self.assertIn("cdp_url", parameters)
        self.assertIn("is_local", parameters)
        self.assertIn("keep_alive", parameters)


if __name__ == "__main__":
    unittest.main()
