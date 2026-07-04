"""@generated DO arm — faithful mirror of the `feature_queries` controller.
`osm.controllers.feature_queries.<is_a>(inp)` free functions (re-exported as
`osm.feature_queries`); standalone, not methods on the model. Call:
`osm.controllers.feature_queries.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`feature_queries:show` — DO arm. Source: FeatureQueriesController#show."""
    raise NotImplementedError("port FeatureQueriesController#show")

