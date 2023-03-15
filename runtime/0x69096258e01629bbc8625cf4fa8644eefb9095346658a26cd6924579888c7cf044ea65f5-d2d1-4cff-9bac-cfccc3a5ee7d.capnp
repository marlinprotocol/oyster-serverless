using Workerd = import "/workerd/workerd.capnp";

    const helloWorldExample :Workerd.Config = (
      services = [ (name = "main", worker = .helloWorld) ],
      sockets = [ ( name = "http", address = "*:45353", http = (), service = "main" ) ]
    );
    
    const helloWorld :Workerd.Worker = (
      serviceWorkerScript = embed "0x69096258e01629bbc8625cf4fa8644eefb9095346658a26cd6924579888c7cf044ea65f5-d2d1-4cff-9bac-cfccc3a5ee7d.js",
      compatibilityDate = "2022-09-16",
    );