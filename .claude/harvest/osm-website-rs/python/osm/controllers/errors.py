"""@generated DO arm — faithful mirror of the `errors` controller.
`osm.controllers.errors.<is_a>(inp)` free functions (re-exported as
`osm.errors`); standalone, not methods on the model. Call:
`osm.controllers.errors.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def bad_request(inp: Input) -> Output:
    """`errors:bad_request` — DO arm. Source: ErrorsController#bad_request."""
    raise NotImplementedError("port ErrorsController#bad_request")

def forbidden(inp: Input) -> Output:
    """`errors:forbidden` — DO arm. Source: ErrorsController#forbidden."""
    raise NotImplementedError("port ErrorsController#forbidden")

def internal_server_error(inp: Input) -> Output:
    """`errors:internal_server_error` — DO arm. Source: ErrorsController#internal_server_error."""
    raise NotImplementedError("port ErrorsController#internal_server_error")

def not_found(inp: Input) -> Output:
    """`errors:not_found` — DO arm. Source: ErrorsController#not_found."""
    raise NotImplementedError("port ErrorsController#not_found")

