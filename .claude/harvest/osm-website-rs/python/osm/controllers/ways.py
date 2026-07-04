"""@generated DO arm — faithful mirror of the `ways` controller.
`osm.controllers.ways.<is_a>(inp)` free functions (re-exported as
`osm.ways`); standalone, not methods on the model. Call:
`osm.controllers.ways.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`ways:create` — DO arm. Source: Api::WaysController#create."""
    raise NotImplementedError("port Api::WaysController#create")

def delete(inp: Input) -> Output:
    """`ways:delete` — DO arm. Source: Api::WaysController#destroy."""
    raise NotImplementedError("port Api::WaysController#destroy")

def list(inp: Input) -> Output:
    """`ways:list` — DO arm. Sources (canonical tile): Api::Nodes::WaysController#index, Api::WaysController#index."""
    raise NotImplementedError("port Api::Nodes::WaysController#index")

def show(inp: Input) -> Output:
    """`ways:show` — DO arm. Sources (canonical tile): Api::WaysController#show, WaysController#show."""
    raise NotImplementedError("port Api::WaysController#show")

def update(inp: Input) -> Output:
    """`ways:update` — DO arm. Source: Api::WaysController#update."""
    raise NotImplementedError("port Api::WaysController#update")

