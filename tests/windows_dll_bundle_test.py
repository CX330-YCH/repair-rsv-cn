import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("bundle_dlls", Path(__file__).resolve().parents[1] / "scripts/bundle-windows-dlls.py")
bundle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bundle)


class DllDependencyTest(unittest.TestCase):
    def test_recursive_case_insensitive_imports_cycles_and_system_apis(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            dlls, system = root / "dlls", root / "system"
            dlls.mkdir(); system.mkdir()
            for name in ["avcodec.dll", "libwinpthread.dll", "unused.dll"]:
                (dlls / name).touch()
            (system / "KERNEL32.dll").touch()
            imports = {
                "helper.exe": ["AVCODEC.dll", "kernel32.DLL", "api-ms-win-core-file-l1-1-0.dll"],
                "avcodec.dll": ["libwinpthread.dll"],
                "libwinpthread.dll": ["avcodec.dll", "KERNEL32.dll"],
            }
            with patch.object(bundle, "imported_dlls", side_effect=lambda path, _: imports[path.name]):
                result = bundle.collect_dependencies(root / "helper.exe", dlls, system, "objdump")
                self.assertEqual([path.name for path in result], ["avcodec.dll", "libwinpthread.dll"])
            with patch.object(bundle, "imported_dlls", return_value=["missing.dll"]):
                with self.assertRaisesRegex(RuntimeError, "missing.dll"):
                    bundle.collect_dependencies(root / "helper.exe", dlls, system, "objdump")


if __name__ == "__main__":
    unittest.main()
