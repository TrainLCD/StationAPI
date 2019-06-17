# This file is responsible for configuring your application
# and its dependencies with the aid of the Mix.Config module.
#
# This configuration file is loaded before any dependency and
# is restricted to this project.

# General application configuration
use Mix.Config

config :station_api,
  ecto_repos: [StationApi.Repo]

# Configures the endpoint
config :station_api, StationApiWeb.Endpoint,
  url: [host: "localhost"],
  secret_key_base: "zq2nSXRVrX7ZucHQ7tN+GkLyGSAacF3JzB+QM0/g1aJt+xgTG6tITZh5w2JYhSCd",
  render_errors: [view: StationApiWeb.ErrorView, accepts: ~w(json)],
  pubsub: [name: StationApi.PubSub, adapter: Phoenix.PubSub.PG2]

# Configures Elixir's Logger
config :logger, :console,
  format: "$time $metadata[$level] $message\n",
  metadata: [:request_id]

# Use Jason for JSON parsing in Phoenix
config :phoenix, :json_library, Jason

# Import environment specific config. This must remain at the bottom
# of this file so it overrides the configuration defined above.
import_config "#{Mix.env()}.exs"
