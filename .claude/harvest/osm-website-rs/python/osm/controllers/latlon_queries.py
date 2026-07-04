"""@generated DO arm — faithful mirror of the `latlon_queries` controller.
`osm.controllers.latlon_queries.<is_a>(inp)` free functions (re-exported as
`osm.latlon_queries`); standalone, not methods on the model. Call:
`osm.controllers.latlon_queries.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`latlon_queries:create` — DO arm. Source: Searches::LatlonQueriesController#create."""
    raise NotImplementedError("port Searches::LatlonQueriesController#create")

