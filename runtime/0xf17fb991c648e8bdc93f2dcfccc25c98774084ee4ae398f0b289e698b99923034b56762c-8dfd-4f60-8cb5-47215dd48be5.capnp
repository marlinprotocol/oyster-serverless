using Workerd = import "/workerd/workerd.capnp";

    const oysterServerlessConfig :Workerd.Config = (
      services = [ (name = "main", worker = .oysterServerless) ],
      sockets = [ ( name = "http", address = "*:40641", http = (), service = "main" ) ]
    );
    
    const oysterServerless :Workerd.Worker = (
      serviceWorkerScript = embed "0xf17fb991c648e8bdc93f2dcfccc25c98774084ee4ae398f0b289e698b99923034b56762c-8dfd-4f60-8cb5-47215dd48be5.js",
      compatibilityDate = "2022-09-16",
    );