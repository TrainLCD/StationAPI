defmodule StationApi.Repo do
  use Ecto.Repo,
    otp_app: :station_api,
    adapter: Ecto.Adapters.MyXQL
end
