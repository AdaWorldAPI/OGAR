"""@generated DO arm — faithful mirror of the `pictures` controller.
`osm.controllers.pictures.<is_a>(inp)` free functions (re-exported as
`osm.pictures`); standalone, not methods on the model. Call:
`osm.controllers.pictures.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`pictures:show` — DO arm. Source: Traces::PicturesController#show."""
    raise NotImplementedError("port Traces::PicturesController#show")

