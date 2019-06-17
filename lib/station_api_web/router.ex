defmodule StationApiWeb.Router do
  use StationApiWeb, :router

  pipeline :api do
    plug :accepts, ["json"]
  end

  scope "/" do
    pipe_through :api

    forward "/graphiql", Absinthe.Plug.GraphiQL,
      schema: StationApiWeb.Schema

    forward "/", Absinthe.Plug,
      schema: StationApiWeb.Schema
  end
end
