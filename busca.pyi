from typing import Optional

class FileMatch:
    """
    Data structure to represent a file that matches the search criteria. It has three attributes:
    ...

    Attributes
    ----------
    path : str
        a string of the path to the file
    percent_match : float
        the ratio of line similarity between the sequences
    lines : str
        the contents of the file
    """

    path: str
    percent_match: float
    lines: str
    def __new__(cls, path: str, percent_match: float, lines: str) -> FileMatch: ...

def search_for_lines(
    reference_string: str,
    search_path: str,
    max_lines: int,
    count: int,
    include_globs: Optional[list[str]] = None,
    exclude_globs: Optional[list[str]] = None,
) -> list[FileMatch]:
    """Search for lines in a search path that match a references string."""
