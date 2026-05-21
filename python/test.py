import inspect
import unittest
from pathlib import Path
from time import perf_counter

import busca_py as busca


def example_usage():
    reference_file_path = "./sample_dir_hello_world/file_1.py"
    with open(reference_file_path, "r") as file:
        reference_string = file.read()

    # Perform a search with required parameters
    all_file_comparisons: list[busca.FileComparison] = busca.search(
        reference_string=reference_string,
        search_path="./sample_dir_hello_world",
    )

    # Comparisons are returned in descending order of similarity_ratio
    closest_file_comparison: busca.FileComparison = all_file_comparisons[0]
    assert closest_file_comparison.path == Path(reference_file_path)
    assert closest_file_comparison.similarity_ratio == 1.0
    assert closest_file_comparison.content == reference_string

    # Perform a search for the top 5 comparisons with additional filters
    # to speed up runtime by skipping files that will not match
    relevant_file_comparisons: list[busca.FileComparison] = busca.search(
        reference_string=reference_string,
        search_path="./sample_dir_hello_world",
        max_file_lines=10_000,
        include_glob=["*.py"],
        count=5,
    )

    assert len(relevant_file_comparisons) < len(all_file_comparisons)

    # Create a new FileComparison object
    new_file_comparison = busca.FileComparison("file/path", 1.0, "file\ncontent")


class TestSignatures(unittest.TestCase):
    def test_module_contains_functions(self):
        expected_functions = {"search"}
        module_attributes = set(dir(busca))
        self.assertTrue(expected_functions.issubset(module_attributes))

    def test_non_empty_search_function_signature(self):
        self.assertTrue(inspect.signature(busca.search).parameters.items())

    def test_non_empty_file_comparison_class_signature(self):
        self.assertTrue(inspect.signature(busca.FileComparison).parameters.items())


class TestSearchResults(unittest.TestCase):
    def setUp(self):
        with open("./sample_dir_hello_world/file_1.py", "r") as file:
            ref_str = file.read()
        self.search = busca.search(
            reference_string=ref_str,
            search_path="./",
            max_file_lines=10000,
            count=5,
            include_glob=["*.py"],
        )

    def test_first_result(self):
        file_comparison = self.search[0]

        expected_content = 'print("Hello World 1")\nprint("Hello World 2")\n\n\nprint("Hello World 3")\nprint("Hello World 4")\n\nprint("Hello World 5")\nprint("Hello World 6")'

        self.assertEqual(file_comparison.path, Path("./sample_dir_hello_world/file_1.py"))
        self.assertEqual(file_comparison.similarity_ratio, 1.0)
        self.assertEqual(file_comparison.content, expected_content)

    def test_third_result(self):
        file_comparison = self.search[2]

        expected_content = '\n\nprint("Hello World 1")\n\nprint("Hello World 3")\n'

        self.assertEqual(
            file_comparison.path,
            Path("./sample_dir_hello_world/nested_dir/sample_python_file_3.py"),
        )
        self.assertEqual(file_comparison.similarity_ratio, 0.4285714328289032)
        self.assertEqual(file_comparison.content, expected_content)

    def test_single_string_include_glob(self):
        with open("./sample_dir_hello_world/file_1.py", "r") as file:
            ref_str = file.read()
        result = busca.search(
            reference_string=ref_str,
            search_path="./sample_dir_hello_world",
            include_glob="*.py",
            count=3,
        )
        self.assertTrue(len(result) >= 1)
        self.assertEqual(result[0].similarity_ratio, 1.0)

    def test_single_string_exclude_glob(self):
        with open("./sample_dir_hello_world/file_1.py", "r") as file:
            ref_str = file.read()
        result = busca.search(
            reference_string=ref_str,
            search_path="./sample_dir_hello_world",
            exclude_glob="*.json",
            count=10,
        )
        for fc in result:
            self.assertFalse(str(fc.path).endswith(".json"))

    def test_glob_type_error_message_includes_inner_detail(self):
        with self.assertRaises(ValueError) as ctx:
            busca.search(
                reference_string="x",
                search_path="./sample_dir_hello_world",
                include_glob=[1, 2],
            )
        msg = str(ctx.exception)
        self.assertIn("glob argument must be", msg)
        self.assertTrue(
            "int" in msg or "argument" in msg or "extract" in msg,
            f"expected inner error detail in message, got: {msg}",
        )


class TestSearchDuration(unittest.TestCase):
    def setUp(self):
        with open("./sample_dir_hello_world/file_1.py", "r") as file:
            self.ref_str = file.read()

    def test_no_globs(self):
        t1 = perf_counter()
        _ = busca.search(
            reference_string=self.ref_str,
            search_path="./",
            max_file_lines=10000,
            count=5,
        )
        duration = perf_counter() - t1
        self.assertLess(duration, 5)

    def test_only_py_files(self):
        t1 = perf_counter()
        _ = busca.search(
            reference_string=self.ref_str,
            search_path="./",
            max_file_lines=10000,
            count=5,
            include_glob=["*.py"],
        )
        duration = perf_counter() - t1
        self.assertLess(duration, 5)


if __name__ == "__main__":
    unittest.main()
    example_usage()
