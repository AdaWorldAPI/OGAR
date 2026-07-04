"""@generated DO arm — faithful mirror of the `redactions` controller.
`osm.controllers.redactions.<is_a>(inp)` free functions (re-exported as
`osm.redactions`); standalone, not methods on the model. Call:
`osm.controllers.redactions.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`redactions:create` — DO arm. Sources (canonical tile): Api::OldElements::RedactionsController#create, RedactionsController#create."""
    raise NotImplementedError("port Api::OldElements::RedactionsController#create")

def delete(inp: Input) -> Output:
    """`redactions:delete` — DO arm. Sources (canonical tile): Api::OldElements::RedactionsController#destroy, RedactionsController#destroy."""
    raise NotImplementedError("port Api::OldElements::RedactionsController#destroy")

def edit(inp: Input) -> Output:
    """`redactions:edit` — DO arm. Source: RedactionsController#edit."""
    raise NotImplementedError("port RedactionsController#edit")

def list(inp: Input) -> Output:
    """`redactions:list` — DO arm. Source: RedactionsController#index."""
    raise NotImplementedError("port RedactionsController#index")

def lookup_old_element(inp: Input) -> Output:
    """`redactions:lookup_old_element` — DO arm. Sources (canonical tile): Api::OldNodes::RedactionsController#lookup_old_element, Api::OldRelations::RedactionsController#lookup_old_element, Api::OldWays::RedactionsController#lookup_old_element."""
    raise NotImplementedError("port Api::OldNodes::RedactionsController#lookup_old_element")

def lookup_redaction(inp: Input) -> Output:
    """`redactions:lookup_redaction` — DO arm. Source: RedactionsController#lookup_redaction."""
    raise NotImplementedError("port RedactionsController#lookup_redaction")

def new_form(inp: Input) -> Output:
    """`redactions:new_form` — DO arm. Source: RedactionsController#new."""
    raise NotImplementedError("port RedactionsController#new")

def show(inp: Input) -> Output:
    """`redactions:show` — DO arm. Source: RedactionsController#show."""
    raise NotImplementedError("port RedactionsController#show")

def update(inp: Input) -> Output:
    """`redactions:update` — DO arm. Source: RedactionsController#update."""
    raise NotImplementedError("port RedactionsController#update")

