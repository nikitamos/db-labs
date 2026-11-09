#:package Aspire.Hosting.PostgreSQL@13.6.1
#:package Aspire.Hosting.Rust@13.6.1-preview.1.26506.6
#:sdk Aspire.AppHost.Sdk@13.6.0
#:property AspireUseCliBundle=true

var builder = DistributedApplication.CreateBuilder(args);

var postgres =
    builder.AddPostgres("postgres")
    .WithHostPort(40737)
    .WithPgAdmin()
    .WithDataVolume();

var db = postgres.AddDatabase("app-db");

builder.AddRustApp("backend", "./app")
    .WithHttpEndpoint(env: "PORT", port: 6200)
    .WithHttpHealthCheck("/health")
    .WithReference(db)
    .WithCertificateTrustScope(CertificateTrustScope.System)
    .WaitFor(db);

builder.Build().Run();
