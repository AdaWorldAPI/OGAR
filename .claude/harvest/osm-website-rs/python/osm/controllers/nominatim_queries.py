"""@generated DO arm — faithful mirror of the `nominatim_queries` controller.
`osm.controllers.nominatim_queries.<is_a>(inp)` free functions (re-exported as
`osm.nominatim_queries`); standalone, not methods on the model. Call:
`osm.controllers.nominatim_queries.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`nominatim_queries:create` — DO arm. Source: Searches::NominatimQueriesController#create."""
    raise NotImplementedError("port Searches::NominatimQueriesController#create")

