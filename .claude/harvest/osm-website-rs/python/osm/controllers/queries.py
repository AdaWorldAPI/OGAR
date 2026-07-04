"""@generated DO arm — faithful mirror of the `queries` controller.
`osm.controllers.queries.<is_a>(inp)` free functions (re-exported as
`osm.queries`); standalone, not methods on the model. Call:
`osm.controllers.queries.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def fetch_text(inp: Input) -> Output:
    """`queries:fetch_text` — DO arm. Source: Searches::QueriesController#fetch_text."""
    raise NotImplementedError("port Searches::QueriesController#fetch_text")

def fetch_xml(inp: Input) -> Output:
    """`queries:fetch_xml` — DO arm. Source: Searches::QueriesController#fetch_xml."""
    raise NotImplementedError("port Searches::QueriesController#fetch_xml")

