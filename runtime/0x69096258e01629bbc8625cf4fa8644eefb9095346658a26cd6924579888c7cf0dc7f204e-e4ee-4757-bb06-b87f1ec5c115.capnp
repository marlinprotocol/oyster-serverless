using Workerd = import "/workerd/workerd.capnp";

    const helloWorldExample :Workerd.Config = (
      services = [ (name = "main", worker = .helloWorld) ],
      sockets = [ ( name = "http", address = "*:46213", http = (), service = "main" ) ]
    );
    
    const helloWorld :Workerd.Worker = (
      serviceWorkerScript = embed "0x69096258e01629bbc8625cf4fa8644eefb9095346658a26cd6924579888c7cf0dc7f204e-e4ee-4757-bb06-b87f1ec5c115.js",
      compatibilityDate = "2022-09-16",
    );