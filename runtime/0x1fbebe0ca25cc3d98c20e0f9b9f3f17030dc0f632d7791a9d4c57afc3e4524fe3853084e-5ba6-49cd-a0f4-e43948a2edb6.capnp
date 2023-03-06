using Workerd = import "/workerd/workerd.capnp";

    const helloWorldExample :Workerd.Config = (
      services = [ (name = "main", worker = .helloWorld) ],
      sockets = [ ( name = "http", address = "*:41875", http = (), service = "main" ) ]
    );
    
    const helloWorld :Workerd.Worker = (
      serviceWorkerScript = embed "0x1fbebe0ca25cc3d98c20e0f9b9f3f17030dc0f632d7791a9d4c57afc3e4524fe3853084e-5ba6-49cd-a0f4-e43948a2edb6.js",
      compatibilityDate = "2022-09-16",
    );