import unittest
from time import perf_counter

import busca_py as busca


def example_usage():
    reference_file_path = "./sample_dir_hello_world/file_1.py"

    with open(reference_file_path, "r") as file:
        reference_string = file.read()

    file_matches = busca.search_for_lines(
        reference_string=reference_string,
        search_path="./",
        max_lines=1000,
        count=5,
        include_globs=["*.py"],
    )

    closest_file_match = file_matches[0]
    assert closest_file_match.path == reference_file_path
    assert closest_file_match.percent_match == 1.0
    assert closest_file_match.lines == reference_string

    new_file_match = busca.FileMatch("file/path", 1.0, "file\ncontent")


class TestSearchResults(unittest.TestCase):
    def setUp(self):
        with open("./sample_dir_hello_world/file_1.py", "r") as file:
            ref_str = file.read()
        self.search = busca.search_for_lines(
            reference_string=ref_str,
            search_path="./",
            max_lines=10000,
            count=5,
            include_globs=["*.py"],
        )

    def test_first_result(self):
        file_match = self.search[0]

        expected_lines = 'print("Hello World 1")\nprint("Hello World 2")\n\n\nprint("Hello World 3")\nprint("Hello World 4")\n\nprint("Hello World 5")\nprint("Hello World 6")'

        self.assertEqual(file_match.path, "./sample_dir_hello_world/file_1.py")
        self.assertEqual(file_match.percent_match, 1.0)
        self.assertEqual(file_match.lines, expected_lines)

    def test_third_result(self):
        file_match = self.search[2]

        expected_lines = '\n\nprint("Hello World 1")\n\nprint("Hello World 3")\n'

        self.assertEqual(file_match.path, "./sample_dir_hello_world/nested_dir/sample_python_file_3.py")
        self.assertEqual(file_match.percent_match, 0.4285714328289032)
        self.assertEqual(file_match.lines, expected_lines)


class TestSearchDuration(unittest.TestCase):
    def setUp(self):
        with open("./sample_dir_hello_world/file_1.py", "r") as file:
            self.ref_str = file.read()

    def test_no_globs(self):
        t1 = perf_counter()
        _ = busca.search_for_lines(
            reference_string=self.ref_str,
            search_path="./",
            max_lines=10000,
            count=5,
        )
        duration = perf_counter() - t1
        self.assertLess(duration, 5)

    def test_only_py_files(self):
        t1 = perf_counter()
        _ = busca.search_for_lines(
            reference_string=self.ref_str,
            search_path="./",
            max_lines=10000,
            count=5,
            include_globs=["*.py"],
        )
        duration = perf_counter() - t1
        self.assertLess(duration, 5)


if __name__ == "__main__":
    example_usage()
    unittest.main()
