from typing import Optional

class FileComparison:
    """
    The result of scoring one candidate file against the reference string.

    Attributes
    ----------
    path : str
        Path to the candidate file.
    similarity_ratio : float
        `similar::TextDiff::ratio()` between the reference and the candidate,
        a Ratcliff/Obershelp similarity in [0.0, 1.0] over the line sequences.
        See ADR-0001.
    content : str
        Full contents of the candidate file.
    """

    path: str
    similarity_ratio: float
    content: str
    def __new__(
        cls, path: str, similarity_ratio: float, content: str
    ) -> FileComparison: ...

def search(
    reference_string: str,
    search_path: str,
    max_file_lines: Optional[int] = None,
    count: Optional[int] = None,
    include_glob: Optional[list[str]] = None,
    exclude_glob: Optional[list[str]] = None,
) -> list[FileComparison]:
    """Walk `search_path` and return a `FileComparison` for each candidate that
    survives the include/exclude globs and `max_file_lines` filter, ranked by
    descending `similarity_ratio`."""
