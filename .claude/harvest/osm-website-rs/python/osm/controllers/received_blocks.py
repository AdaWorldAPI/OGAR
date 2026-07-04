"""@generated DO arm — faithful mirror of the `received_blocks` controller.
`osm.controllers.received_blocks.<is_a>(inp)` free functions (re-exported as
`osm.received_blocks`); standalone, not methods on the model. Call:
`osm.controllers.received_blocks.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def delete(inp: Input) -> Output:
    """`received_blocks:delete` — DO arm. Source: Users::ReceivedBlocksController#destroy."""
    raise NotImplementedError("port Users::ReceivedBlocksController#destroy")

def edit(inp: Input) -> Output:
    """`received_blocks:edit` — DO arm. Source: Users::ReceivedBlocksController#edit."""
    raise NotImplementedError("port Users::ReceivedBlocksController#edit")

def show(inp: Input) -> Output:
    """`received_blocks:show` — DO arm. Source: Users::ReceivedBlocksController#show."""
    raise NotImplementedError("port Users::ReceivedBlocksController#show")

