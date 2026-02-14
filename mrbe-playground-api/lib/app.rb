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
end

$APP = App.new