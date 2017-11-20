
// Create the Error, ErrorKind, ResultExt, and Result types
error_chain!{
    foreign_links {
        Log(::log::SetLoggerError);
        Notify(::notify::Error);
        Ssh(::ssh2::Error);
        Tcp(::std::io::Error);
        Env(::std::env::VarError);
        Ignore(::ignore::Error);
        StripPrefixError(::std::path::StripPrefixError);
    }

    errors {
        EnviromentRead(env_variable: String) {
            description("Failed to read enviroment variable")
            display("Unable to read enviroment variable `{}`", env_variable)
        }

        HostConnection(host: String) {
            description("Failed to connect to host")
            display("Unable to connect to host `{}`", host)
        }

        UserAuthentication(user: String, host: String) {
            description("Failed to authenticate user with host")
            display("Unable to authenticate user `{}` with host `{}`", user, host)
        }

        Mkdir(path: String) {
            description("Failed to authenticate create directory")
            display("Unable to create directory `{}`", path)
        }

        DirectoryExists(path: String) {
            description("Failed to create directory")
            display("Directory `{}` already exists", path)
        }

        IsDirectory(path: String) {
            description("Failed to read file")
            display("Path `{}` is a directory", path)
        }

        LStat(path: String) {
            description("Failed to run lstat")
            display("Unable to run lstat on path `{}`", path)
        }

        InvalidUTF8(path: String) {
            description("Stream did not contain valid UTF-8")
            display("Unable to get a UTF-8 stream for `{}`", path)
        }

        RelativePathGeneration(directory: String, path: String) {
            description("Path was not able to be tranformed into a relative one")
            display("Unable to transform path {} into relative using {}", path, directory)
        }
    }
}
