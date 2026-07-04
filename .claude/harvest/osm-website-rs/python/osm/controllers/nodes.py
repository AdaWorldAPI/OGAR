"""@generated DO arm — faithful mirror of the `nodes` controller.
`osm.controllers.nodes.<is_a>(inp)` free functions (re-exported as
`osm.nodes`); standalone, not methods on the model. Call:
`osm.controllers.nodes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`nodes:create` — DO arm. Source: Api::NodesController#create."""
    raise NotImplementedError("port Api::NodesController#create")

def delete(inp: Input) -> Output:
    """`nodes:delete` — DO arm. Source: Api::NodesController#destroy."""
    raise NotImplementedError("port Api::NodesController#destroy")

def list(inp: Input) -> Output:
    """`nodes:list` — DO arm. Source: Api::NodesController#index."""
    raise NotImplementedError("port Api::NodesController#index")

def show(inp: Input) -> Output:
    """`nodes:show` — DO arm. Sources (canonical tile): Api::NodesController#show, NodesController#show."""
    raise NotImplementedError("port Api::NodesController#show")

def update(inp: Input) -> Output:
    """`nodes:update` — DO arm. Source: Api::NodesController#update."""
    raise NotImplementedError("port Api::NodesController#update")

