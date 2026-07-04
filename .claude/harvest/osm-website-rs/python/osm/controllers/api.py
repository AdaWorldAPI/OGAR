"""@generated DO arm — faithful mirror of the `api` controller.
`osm.controllers.api.<is_a>(inp)` free functions (re-exported as
`osm.api`); standalone, not methods on the model. Call:
`osm.controllers.api.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def api_call_handle_error(inp: Input) -> Output:
    """`api:api_call_handle_error` — DO arm. Source: ApiController#api_call_handle_error."""
    raise NotImplementedError("port ApiController#api_call_handle_error")

def api_call_timeout(inp: Input) -> Output:
    """`api:api_call_timeout` — DO arm. Source: ApiController#api_call_timeout."""
    raise NotImplementedError("port ApiController#api_call_timeout")

def authorize(inp: Input) -> Output:
    """`api:authorize` — DO arm. Source: ApiController#authorize."""
    raise NotImplementedError("port ApiController#authorize")

def check_rate_limit(inp: Input) -> Output:
    """`api:check_rate_limit` — DO arm. Source: ApiController#check_rate_limit."""
    raise NotImplementedError("port ApiController#check_rate_limit")

def current_ability(inp: Input) -> Output:
    """`api:current_ability` — DO arm. Source: ApiController#current_ability."""
    raise NotImplementedError("port ApiController#current_ability")

def deny_access(inp: Input) -> Output:
    """`api:deny_access` — DO arm. Source: ApiController#deny_access."""
    raise NotImplementedError("port ApiController#deny_access")

def gpx_status(inp: Input) -> Output:
    """`api:gpx_status` — DO arm. Source: ApiController#gpx_status."""
    raise NotImplementedError("port ApiController#gpx_status")

def scope_enabled(inp: Input) -> Output:
    """`api:scope_enabled?` — DO arm. Source: ApiController#scope_enabled?."""
    raise NotImplementedError("port ApiController#scope_enabled?")

def set_request_formats(inp: Input) -> Output:
    """`api:set_request_formats` — DO arm. Source: ApiController#set_request_formats."""
    raise NotImplementedError("port ApiController#set_request_formats")

def setup_user_auth(inp: Input) -> Output:
    """`api:setup_user_auth` — DO arm. Source: ApiController#setup_user_auth."""
    raise NotImplementedError("port ApiController#setup_user_auth")

