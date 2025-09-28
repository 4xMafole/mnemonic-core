FROM mcr.microsoft.com/devcontainers/base:ubuntu

#INSTALL SYSTEM DEPENDENCIES
# This command tells the underlying LInux system to install the
# 'clang' C/C++ compiler and the developer files for it ('libclang-dev')
# 'build-essential' is a handy package that includes other useful tools.
RUN apt-get update && apt-get install -y clang libclang-dev build-essential
# Set the working directory for Codespaces
WORKDIR /workspaces
