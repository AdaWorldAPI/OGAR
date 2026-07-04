"""@generated DO arm — faithful mirror of the `issued_blocks` controller.
`osm.controllers.issued_blocks.<is_a>(inp)` free functions (re-exported as
`osm.issued_blocks`); standalone, not methods on the model. Call:
`osm.controllers.issued_blocks.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`issued_blocks:show` — DO arm. Source: Users::IssuedBlocksController#show."""
    raise NotImplementedError("port Users::IssuedBlocksController#show")

