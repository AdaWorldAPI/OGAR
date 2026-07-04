"""@generated DO arm — faithful mirror of the `user_preferences` controller.
`osm.controllers.user_preferences.<is_a>(inp)` free functions (re-exported as
`osm.user_preferences`); standalone, not methods on the model. Call:
`osm.controllers.user_preferences.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def delete(inp: Input) -> Output:
    """`user_preferences:delete` — DO arm. Source: Api::UserPreferencesController#destroy."""
    raise NotImplementedError("port Api::UserPreferencesController#destroy")

def list(inp: Input) -> Output:
    """`user_preferences:list` — DO arm. Source: Api::UserPreferencesController#index."""
    raise NotImplementedError("port Api::UserPreferencesController#index")

def show(inp: Input) -> Output:
    """`user_preferences:show` — DO arm. Source: Api::UserPreferencesController#show."""
    raise NotImplementedError("port Api::UserPreferencesController#show")

def update(inp: Input) -> Output:
    """`user_preferences:update` — DO arm. Source: Api::UserPreferencesController#update."""
    raise NotImplementedError("port Api::UserPreferencesController#update")

def update_all(inp: Input) -> Output:
    """`user_preferences:update_all` — DO arm. Source: Api::UserPreferencesController#update_all."""
    raise NotImplementedError("port Api::UserPreferencesController#update_all")

