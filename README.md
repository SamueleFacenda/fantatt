# Fantasy tabletennis
One project, one monorepo, 3 modules, 3 build system

> [!IMPORTANT]  
> Very WIP, just an idea for now.

[![built with nix](https://builtwithnix.org/badge.svg)](https://builtwithnix.org)

## info

read the docs (in /docs).

## A simple scraper for the fitet result portal
Not so simple in the end, features:
- multithreading (more than 20 threads takes down the website)
- persistency (data are saved in a general 
relational db, for a complete list of the choices just look 
at [sqlalchemy](https://sqlalchemy.org))
- db update operation optimization
- cli utility
- json saving option ([here](https://github.com/SamueleFacenda/Python-scripts/commits/13be8f5def93a9322ecd51e21679c25aa3a48a82))

![db scheme](assets/db_scheme.jpg)

TODO:
- [ ] web ui for fantasy tabletennis
- [ ] player auction
- [ ] formation management (choose 3 players) 
- [ ] score assignment (should think about something smart)
- [ ] match simulation (A-X and score comparison)
- [ ] update fitet parser request discharging (some tournaments are present 
but not completely updated)

## CONTRIBUTING
I don't really think there is someone out there
interested in contributing to this project,
but if you are (awesome), just open an issue or a PR.
Every contribution is welcome, and also 
we can discuss the game rules together.

 
https://github.com/NixOS/nixpkgs/blob/c1f0be03736e6d5ab4d19e867e6684686203eee8/pkgs/games/mindustry/default.nix#L127
https://www.baeldung.com/spring-mvc-static-resources
