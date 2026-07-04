"""@generated DO arm — faithful mirror of the `issues` controller.
`osm.controllers.issues.<is_a>(inp)` free functions (re-exported as
`osm.issues`); standalone, not methods on the model. Call:
`osm.controllers.issues.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def find_issue(inp: Input) -> Output:
    """`issues:find_issue` — DO arm. Source: IssuesController#find_issue."""
    raise NotImplementedError("port IssuesController#find_issue")

def ignore(inp: Input) -> Output:
    """`issues:ignore` — DO arm. Source: IssuesController#ignore."""
    raise NotImplementedError("port IssuesController#ignore")

def list(inp: Input) -> Output:
    """`issues:list` — DO arm. Source: IssuesController#index."""
    raise NotImplementedError("port IssuesController#index")

def reopen(inp: Input) -> Output:
    """`issues:reopen` — DO arm. Source: IssuesController#reopen."""
    raise NotImplementedError("port IssuesController#reopen")

def resolve(inp: Input) -> Output:
    """`issues:resolve` — DO arm. Source: IssuesController#resolve."""
    raise NotImplementedError("port IssuesController#resolve")

def show(inp: Input) -> Output:
    """`issues:show` — DO arm. Source: IssuesController#show."""
    raise NotImplementedError("port IssuesController#show")

