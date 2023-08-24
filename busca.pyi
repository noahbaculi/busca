from typing import Optional

class FileMatch:
    path: str
    percent_match: float
    lines: str

def search_for_lines(
    reference_string: str,
    search_path: str,
    max_lines: int,
    count: int,
    include_globs: Optional[list[str]] = None,
    exclude_globs: Optional[list[str]] = None,
) -> list[FileMatch]: ...
