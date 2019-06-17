defmodule StationApiWeb.Router do
  use StationApiWeb, :router

  pipeline :api do
    plug :accepts, ["json"]
  end

  scope "/api", StationApiWeb do
    pipe_through :api
  end
end
