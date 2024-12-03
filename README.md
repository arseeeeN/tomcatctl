# tomcatctl

`tomcatlctl` is a tool for controlling Tomcat Apache from the command line and automating deployment of Java apps.

## How to use

First you need to create a config for the deployment you want to create.

```sh
# tomcatctl config add <config-name> <deployment-path> <project-path>
#   - config-name: The name of the config, has no impact on the deployment
#   - deployment-path: The path to which the project will be deployed in Tomcat
#   - project-path: The location of your target folder with the *.war file inside. Supports glob paths.
tomcatctl config add magnolia /dev ./*-webapp
```

After that you can deploy your built Java project.

```sh
# tomcatctl run <config-name>
tomcatctl run magnolia
```

## Why does this exist?

I wanted to have a way of cleanly deploying Java projects to Tomcat without having to rely on tools like IntelliJ to do it for me.
After some investigation I couldn't find a single tool that does exactly what I want, so I built one myself.
This can also potentially be used in pipelines and containers to more cleanly deploy tomcat projects.

