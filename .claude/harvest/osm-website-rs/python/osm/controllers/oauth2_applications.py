"""@generated DO arm — faithful mirror of the `oauth2_applications` controller.
`osm.controllers.oauth2_applications.<is_a>(inp)` free functions (re-exported as
`osm.oauth2_applications`); standalone, not methods on the model. Call:
`osm.controllers.oauth2_applications.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def application_params(inp: Input) -> Output:
    """`oauth2_applications:application_params` — DO arm. Source: Oauth2ApplicationsController#application_params."""
    raise NotImplementedError("port Oauth2ApplicationsController#application_params")

def list(inp: Input) -> Output:
    """`oauth2_applications:list` — DO arm. Source: Oauth2ApplicationsController#index."""
    raise NotImplementedError("port Oauth2ApplicationsController#index")

def set_application(inp: Input) -> Output:
    """`oauth2_applications:set_application` — DO arm. Source: Oauth2ApplicationsController#set_application."""
    raise NotImplementedError("port Oauth2ApplicationsController#set_application")

