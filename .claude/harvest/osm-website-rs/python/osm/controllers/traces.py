"""@generated DO arm — faithful mirror of the `traces` controller.
`osm.controllers.traces.<is_a>(inp)` free functions (re-exported as
`osm.traces`); standalone, not methods on the model. Call:
`osm.controllers.traces.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`traces:create` — DO arm. Sources (canonical tile): Api::TracesController#create, TracesController#create."""
    raise NotImplementedError("port Api::TracesController#create")

def default_visibility(inp: Input) -> Output:
    """`traces:default_visibility` — DO arm. Source: TracesController#default_visibility."""
    raise NotImplementedError("port TracesController#default_visibility")

def delete(inp: Input) -> Output:
    """`traces:delete` — DO arm. Sources (canonical tile): Api::TracesController#destroy, TracesController#destroy."""
    raise NotImplementedError("port Api::TracesController#destroy")

def do_create(inp: Input) -> Output:
    """`traces:do_create` — DO arm. Sources (canonical tile): Api::TracesController#do_create, TracesController#do_create."""
    raise NotImplementedError("port Api::TracesController#do_create")

def edit(inp: Input) -> Output:
    """`traces:edit` — DO arm. Source: TracesController#edit."""
    raise NotImplementedError("port TracesController#edit")

def list(inp: Input) -> Output:
    """`traces:list` — DO arm. Sources (canonical tile): Api::Users::TracesController#index, TracesController#index."""
    raise NotImplementedError("port Api::Users::TracesController#index")

def mine(inp: Input) -> Output:
    """`traces:mine` — DO arm. Source: TracesController#mine."""
    raise NotImplementedError("port TracesController#mine")

def new_form(inp: Input) -> Output:
    """`traces:new_form` — DO arm. Source: TracesController#new."""
    raise NotImplementedError("port TracesController#new")

def offline_error(inp: Input) -> Output:
    """`traces:offline_error` — DO arm. Source: Api::TracesController#offline_error."""
    raise NotImplementedError("port Api::TracesController#offline_error")

def offline_redirect(inp: Input) -> Output:
    """`traces:offline_redirect` — DO arm. Source: TracesController#offline_redirect."""
    raise NotImplementedError("port TracesController#offline_redirect")

def offline_warning(inp: Input) -> Output:
    """`traces:offline_warning` — DO arm. Source: TracesController#offline_warning."""
    raise NotImplementedError("port TracesController#offline_warning")

def show(inp: Input) -> Output:
    """`traces:show` — DO arm. Sources (canonical tile): Api::TracesController#show, TracesController#show."""
    raise NotImplementedError("port Api::TracesController#show")

def trace_params(inp: Input) -> Output:
    """`traces:trace_params` — DO arm. Source: TracesController#trace_params."""
    raise NotImplementedError("port TracesController#trace_params")

def update(inp: Input) -> Output:
    """`traces:update` — DO arm. Sources (canonical tile): Api::TracesController#update, TracesController#update."""
    raise NotImplementedError("port Api::TracesController#update")

