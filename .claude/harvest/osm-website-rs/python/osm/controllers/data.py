"""@generated DO arm — faithful mirror of the `data` controller.
`osm.controllers.data.<is_a>(inp)` free functions (re-exported as
`osm.data`); standalone, not methods on the model. Call:
`osm.controllers.data.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def offline_error(inp: Input) -> Output:
    """`data:offline_error` — DO arm. Source: Api::Traces::DataController#offline_error."""
    raise NotImplementedError("port Api::Traces::DataController#offline_error")

def offline_redirect(inp: Input) -> Output:
    """`data:offline_redirect` — DO arm. Source: Traces::DataController#offline_redirect."""
    raise NotImplementedError("port Traces::DataController#offline_redirect")

def show(inp: Input) -> Output:
    """`data:show` — DO arm. Sources (canonical tile): Api::Traces::DataController#show, Traces::DataController#show."""
    raise NotImplementedError("port Api::Traces::DataController#show")

