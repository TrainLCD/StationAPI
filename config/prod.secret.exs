# In this file, we load production configuration and
# secrets from environment variables. You can also
# hardcode secrets, although such is generally not
# recommended and you have to remember to add this
# file to your .gitignore.
use Mix.Config

database_hostname =
  System.get_env("DATABASE_HOST") ||
    raise """
    environment variable DATABASE_HOST is missing.
    """

database_user =
  System.get_env("DATABASE_USER") ||
    raise """
    environment variable DATABASE_USER is missing.
    """

database_password =
  System.get_env("DATABASE_PASSWORD") ||
    raise """
    environment variable DATABASE_PASSWORD is missing.
    """

database_name =
  System.get_env("DATABASE_NAME") ||
    raise """
    environment variable DATABASE_NAME is missing.
    """

config :station_api, StationApi.Repo,
  username: database_user,
  password: database_password,
  database: database_name,
  sync_connect: true,
  hostname: database_hostname,
  backoff_type: :stop,
  pool_size: String.to_integer(System.get_env("POOL_SIZE") || "20")

secret_key_base =
  System.get_env("SECRET_KEY_BASE") ||
    raise """
    environment variable SECRET_KEY_BASE is missing.
    You can generate one by calling: mix phx.gen.secret
    """

config :station_api, StationApiWeb.Endpoint,
  http: [:inet6, port: String.to_integer(System.get_env("PORT") || "4000")],
  secret_key_base: secret_key_base
