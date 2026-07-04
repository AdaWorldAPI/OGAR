"""@generated DO arm — faithful mirror of the `users` controller.
`osm.controllers.users.<is_a>(inp)` free functions (re-exported as
`osm.users`); standalone, not methods on the model. Call:
`osm.controllers.users.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def auth_failure(inp: Input) -> Output:
    """`users:auth_failure` — DO arm. Source: UsersController#auth_failure."""
    raise NotImplementedError("port UsersController#auth_failure")

def auth_success(inp: Input) -> Output:
    """`users:auth_success` — DO arm. Source: UsersController#auth_success."""
    raise NotImplementedError("port UsersController#auth_success")

def check_signup_allowed(inp: Input) -> Output:
    """`users:check_signup_allowed?` — DO arm. Source: UsersController#check_signup_allowed?."""
    raise NotImplementedError("port UsersController#check_signup_allowed?")

def create(inp: Input) -> Output:
    """`users:create` — DO arm. Source: UsersController#create."""
    raise NotImplementedError("port UsersController#create")

def details(inp: Input) -> Output:
    """`users:details` — DO arm. Source: Api::UsersController#details."""
    raise NotImplementedError("port Api::UsersController#details")

def go_public(inp: Input) -> Output:
    """`users:go_public` — DO arm. Source: UsersController#go_public."""
    raise NotImplementedError("port UsersController#go_public")

def list(inp: Input) -> Output:
    """`users:list` — DO arm. Source: Api::UsersController#index."""
    raise NotImplementedError("port Api::UsersController#index")

def new_form(inp: Input) -> Output:
    """`users:new_form` — DO arm. Source: UsersController#new."""
    raise NotImplementedError("port UsersController#new")

def save_new_user(inp: Input) -> Output:
    """`users:save_new_user` — DO arm. Source: UsersController#save_new_user."""
    raise NotImplementedError("port UsersController#save_new_user")

def show(inp: Input) -> Output:
    """`users:show` — DO arm. Sources (canonical tile): Api::UsersController#show, UsersController#show."""
    raise NotImplementedError("port Api::UsersController#show")

def user_params(inp: Input) -> Output:
    """`users:user_params` — DO arm. Source: UsersController#user_params."""
    raise NotImplementedError("port UsersController#user_params")

def valid_turnstile_response(inp: Input) -> Output:
    """`users:valid_turnstile_response?` — DO arm. Source: UsersController#valid_turnstile_response?."""
    raise NotImplementedError("port UsersController#valid_turnstile_response?")

def welcome_options(inp: Input) -> Output:
    """`users:welcome_options` — DO arm. Source: UsersController#welcome_options."""
    raise NotImplementedError("port UsersController#welcome_options")

