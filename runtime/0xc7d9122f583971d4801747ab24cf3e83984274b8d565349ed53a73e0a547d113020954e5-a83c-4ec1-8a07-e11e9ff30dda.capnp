using Workerd = import "/workerd/workerd.capnp";

    const helloWorldExample :Workerd.Config = (
      services = [ (name = "main", worker = .helloWorld) ],
      sockets = [ ( name = "http", address = "*:38643", http = (), service = "main" ) ]
    );
    
    const helloWorld :Workerd.Worker = (
      serviceWorkerScript = embed "0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113020954e5-a83c-4ec1-8a07-e11e9ff30dda.js",
      compatibilityDate = "2022-09-16",
    );