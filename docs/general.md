# General architecture of fantatt

You should already know how tabletennis (fitet rules) and a fantasy league works.
The project is divided in three parts:
- the frontend (react js)
- the backend (spring boot kotlin)
- the data fetcher (python)


## Data fetching
The data are fetched from the fitet official results portal. Since there is not an API and the entire website use 
legacy technologies like framesets and so, it's a mess of html parsing, web scraping, codes and ids moving.

The implementation uses python requests and beautifulsoup4 to get the data and sqlalchemy to persist them. 
Quick updating is supported (fetch just the changes) and persistent caching is used where possible.

## Data sources (DB)
There are two db, the one with the data from the fitet portal and the application data one (users, fantasy league data).
The first is created with sqlalchemy orm and I plan to use it from spring boot with the spring data JDBC library 
(language agnostic sql). The latter should be created and managed with the spring data JPA orm. In spring boot is easy to define
more data repositories.

## Infra
Nixos!!
All the parts are already ready to be packaged with nix, so the deployment shoud be fast and easy.
Defining the server configuration with nixos shoud also be very easy for me (containers).

## Frontend deployment
Requirements: caching of the statis folder (react output with hash names)
Possibilities:
- nginx reverse proxy, mask the api and the react assets under one endopoint
- spring boot assets serving, configurable via env vars, supports caching but not so configurable
(I cannot cache the index.html)
