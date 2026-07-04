"""@generated DO arm — faithful mirror of the `user_blocks` controller.
`osm.controllers.user_blocks.<is_a>(inp)` free functions (re-exported as
`osm.user_blocks`); standalone, not methods on the model. Call:
`osm.controllers.user_blocks.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`user_blocks:create` — DO arm. Sources (canonical tile): Api::UserBlocksController#create, UserBlocksController#create."""
    raise NotImplementedError("port Api::UserBlocksController#create")

def edit(inp: Input) -> Output:
    """`user_blocks:edit` — DO arm. Source: UserBlocksController#edit."""
    raise NotImplementedError("port UserBlocksController#edit")

def list(inp: Input) -> Output:
    """`user_blocks:list` — DO arm. Source: UserBlocksController#index."""
    raise NotImplementedError("port UserBlocksController#index")

def lookup_user_block(inp: Input) -> Output:
    """`user_blocks:lookup_user_block` — DO arm. Source: UserBlocksController#lookup_user_block."""
    raise NotImplementedError("port UserBlocksController#lookup_user_block")

def new_form(inp: Input) -> Output:
    """`user_blocks:new_form` — DO arm. Source: UserBlocksController#new."""
    raise NotImplementedError("port UserBlocksController#new")

def require_valid_params(inp: Input) -> Output:
    """`user_blocks:require_valid_params` — DO arm. Source: UserBlocksController#require_valid_params."""
    raise NotImplementedError("port UserBlocksController#require_valid_params")

def show(inp: Input) -> Output:
    """`user_blocks:show` — DO arm. Sources (canonical tile): Api::UserBlocksController#show, UserBlocksController#show."""
    raise NotImplementedError("port Api::UserBlocksController#show")

def update(inp: Input) -> Output:
    """`user_blocks:update` — DO arm. Source: UserBlocksController#update."""
    raise NotImplementedError("port UserBlocksController#update")

