"""@generated DO arm — faithful mirror of the `relations` controller.
`osm.controllers.relations.<is_a>(inp)` free functions (re-exported as
`osm.relations`); standalone, not methods on the model. Call:
`osm.controllers.relations.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`relations:create` — DO arm. Source: Api::RelationsController#create."""
    raise NotImplementedError("port Api::RelationsController#create")

def delete(inp: Input) -> Output:
    """`relations:delete` — DO arm. Source: Api::RelationsController#destroy."""
    raise NotImplementedError("port Api::RelationsController#destroy")

def list(inp: Input) -> Output:
    """`relations:list` — DO arm. Sources (canonical tile): Api::Nodes::RelationsController#index, Api::Relations::RelationsController#index, Api::RelationsController#index, Api::Ways::RelationsController#index."""
    raise NotImplementedError("port Api::Nodes::RelationsController#index")

def show(inp: Input) -> Output:
    """`relations:show` — DO arm. Sources (canonical tile): Api::RelationsController#show, RelationsController#show."""
    raise NotImplementedError("port Api::RelationsController#show")

def update(inp: Input) -> Output:
    """`relations:update` — DO arm. Source: Api::RelationsController#update."""
    raise NotImplementedError("port Api::RelationsController#update")

