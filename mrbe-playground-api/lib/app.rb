class App < Uzumibi::Router
  get "/" do |req, res|
    debug_console("[Uzumibi] Received request at /")
    res.status_code = 200
    res.headers = {
      "Content-Type" => "application/json",
      "X-Powered-By" => "#{RUBY_ENGINE} #{RUBY_VERSION}"
    }
    res.body = JSON.generate({
      message: "Hello from mrbe-playground-api!",
      ruby_engine: RUBY_ENGINE,
      ruby_version: RUBY_VERSION,
    })
    res
  end

  get "/kv" do |req, res|
    res.status_code = 200
    res.headers = {
      "Content-Type" => "application/json",
      "X-Powered-By" => "#{RUBY_ENGINE} #{RUBY_VERSION}",
      "Access-Control-Allow-Origin" => "https://mrubyedge.github.io",
      "Access-Control-Allow-Methods" => "GET, POST, OPTIONS",
      "Access-Control-Allow-Headers" => "Content-Type"
    }

    debug_console("[Uzumibi] Received request at /kv with params: #{req.params.inspect}")
    key = req.params[:key]
    value = Uzumibi::KV.get(key)

    res.body = JSON.generate({
      key: key,
      value: value,
    })
    res
  end

  post "/kv" do |req, res|
    res.status_code = 200
    res.headers = {
      "Content-Type" => "application/json",
      "X-Powered-By" => "#{RUBY_ENGINE} #{RUBY_VERSION}",
      "Access-Control-Allow-Origin" => "https://mrubyedge.github.io",
      "Access-Control-Allow-Methods" => "GET, POST, OPTIONS",
      "Access-Control-Allow-Headers" => "Content-Type"
    }
    # TODO: check request content-type

    debug_console("[Uzumibi] Received request at /kv with params: #{req.params.inspect}")
    key = req.params[:key]
    value = req.params[:value]
    Uzumibi::KV.set(key, value)

    res.body = JSON.generate({
      key: key,
      ok: true,
    })
    res
  end

  # options "/kv" do |req, res|
  #   res.status_code = 204
  #   res.headers = {
  #     "Access-Control-Allow-Origin" => "https://mrubyedge.github.io",
  #     "Access-Control-Allow-Methods" => "GET, POST, OPTIONS",
  #     "Access-Control-Allow-Headers" => "Content-Type",
  #     "Access-Control-Max-Age" => "86400"
  #   }
  #   res.body = ""
  #   res
  # end
end

$APP = App.new