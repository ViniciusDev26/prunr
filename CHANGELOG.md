# Changelog

## [0.3.0](https://github.com/ViniciusDev26/prunr/compare/v0.2.1...v0.3.0) (2026-11-25)


### Features

* monorepos are one graph with dependencies checked per package ([#33](https://github.com/ViniciusDev26/prunr/issues/33)) ([7be094f](https://github.com/ViniciusDev26/prunr/commit/7be094f53bb8c7b2e49145966b2083a3e572283e))
* next.js routes and the nestjs main are entries by convention ([#32](https://github.com/ViniciusDev26/prunr/issues/32)) ([9611941](https://github.com/ViniciusDev26/prunr/commit/96119417ff90923fe56d95e78a1925ce044b3c1d))
* prunr fix deletes the declaration an export leaves unused ([#31](https://github.com/ViniciusDev26/prunr/issues/31)) ([bb6170c](https://github.com/ViniciusDev26/prunr/commit/bb6170c87ba334efb255a77b93d6292eb6a24143))


### Documentation

* roadmap items for fix declarations, test-only, plugin toggles and name-loaded packages ([#29](https://github.com/ViniciusDev26/prunr/issues/29)) ([b36b719](https://github.com/ViniciusDev26/prunr/commit/b36b7190eb53c60ff9760a02a2fb30b061a98482))

## [0.2.1](https://github.com/ViniciusDev26/prunr/compare/v0.2.0...v0.2.1) (2026-11-25)


### Bug Fixes

* a jsdoc import type uses its module ([#27](https://github.com/ViniciusDev26/prunr/issues/27)) ([6faa4fd](https://github.com/ViniciusDev26/prunr/commit/6faa4fd57e9156e9055c6fa80fdc70005f539162))

## [0.2.0](https://github.com/ViniciusDev26/prunr/compare/v0.1.0...v0.2.0) (2026-11-25)


### Features

* a package the scripts of package.json run is used ([#19](https://github.com/ViniciusDev26/prunr/issues/19)) ([1662365](https://github.com/ViniciusDev26/prunr/commit/16623650469764cf7d945312daa52fefe7484b5c))
* a require built from a relative prefix uses every file it can reach ([#18](https://github.com/ViniciusDev26/prunr/issues/18)) ([e12ca9f](https://github.com/ViniciusDev26/prunr/commit/e12ca9f83e238c864b2be17148ce57e305bacf14))
* github annotations and findings new since a commit ([#25](https://github.com/ViniciusDev26/prunr/issues/25)) ([7254ee4](https://github.com/ViniciusDev26/prunr/commit/7254ee41a272b22cd988141036be6878ba1ac243))
* ignore an export or a file with a comment, and report a comment that ignores nothing ([#24](https://github.com/ViniciusDev26/prunr/issues/24)) ([a84fbf8](https://github.com/ViniciusDev26/prunr/commit/a84fbf82eb75ca385458fa9625e9b4406f5c32a9))
* jest and vitest find their own tests, config and setup files ([#22](https://github.com/ViniciusDev26/prunr/issues/22)) ([5011821](https://github.com/ViniciusDev26/prunr/commit/5011821677fd6e5a209814cc911c7fae53d93921))
* prunr fix removes what check reports, round after round ([#26](https://github.com/ViniciusDev26/prunr/issues/26)) ([affda30](https://github.com/ViniciusDev26/prunr/commit/affda301467e3ea3f9cdaf759e470df736da8bc8))
* read esm and typescript in the same graph as commonjs ([#21](https://github.com/ViniciusDev26/prunr/issues/21)) ([a1a0590](https://github.com/ViniciusDev26/prunr/commit/a1a059089095c26b162dbfaa6177ab5d022c0844))


### Documentation

* add brand identity and logo ([#15](https://github.com/ViniciusDev26/prunr/issues/15)) ([db6e26e](https://github.com/ViniciusDev26/prunr/commit/db6e26ea08ca2264d39824127fb1ebf8e86824bf))
* add the roadmap after the mvp ([#16](https://github.com/ViniciusDev26/prunr/issues/16)) ([9bf1102](https://github.com/ViniciusDev26/prunr/commit/9bf11023b0478cafb9b94953c731a6e40d1452a5))

## 0.1.0 (2026-11-25)


### Features

* a baseline that only shrinks ([#6](https://github.com/ViniciusDev26/prunr/issues/6)) ([889d58b](https://github.com/ViniciusDev26/prunr/commit/889d58bf973885bf1fed52d64252403b257435bc))
* config schema, ignored dependencies and prunr init ([#5](https://github.com/ViniciusDev26/prunr/issues/5)) ([afecaaa](https://github.com/ViniciusDev26/prunr/commit/afecaaa578459cc48703fe0f571e88e1dd8315ed))
* **config:** load and validate prunr.json ([a9649f3](https://github.com/ViniciusDev26/prunr/commit/a9649f32e117bf436bb38130b984d8ea265e2648))
* **facts:** read requires, their uses and exports from a CommonJS file ([8a532f7](https://github.com/ViniciusDev26/prunr/commit/8a532f7dfb2b7ac4479c024e67ace57149a6f682))
* ponto 1 do MVP — um grafo só ([ee7fb7c](https://github.com/ViniciusDev26/prunr/commit/ee7fb7c9b7a79daa9aa7e0658df02e2c3da47476))
* **project:** one module graph, each file parsed once ([49444fa](https://github.com/ViniciusDev26/prunr/commit/49444fabc881b3050c1aafbeaa6b512af4cf2e83))
* prunr check runs every check over one graph ([70ce3f0](https://github.com/ViniciusDev26/prunr/commit/70ce3f0a4769fcca0917f1e51d7873cb3c28ac20))
* report unused imports, leave other unused locals to biome ([#7](https://github.com/ViniciusDev26/prunr/issues/7)) ([13257dd](https://github.com/ViniciusDev26/prunr/commit/13257ddcb5bb1b191e92d4a0ba6e530762893190))
* **resolve:** resolve specifiers like Node.js and name packages ([410817e](https://github.com/ViniciusDev26/prunr/commit/410817e894b17064ac0685a63cb3b919c6b8a07b))
* tests keep nothing alive ([#4](https://github.com/ViniciusDev26/prunr/issues/4)) ([fd6a25d](https://github.com/ViniciusDev26/prunr/commit/fd6a25d8058a952622e742bc104b3f0a9e9005a8))


### Bug Fixes

* publish the main npm package as @prunr/prunr ([#11](https://github.com/ViniciusDev26/prunr/issues/11)) ([c8814ba](https://github.com/ViniciusDev26/prunr/commit/c8814ba9ed6ca62d61ddbc860caa0685e7059821))


### Documentation

* commits and pull requests are written in English ([133be31](https://github.com/ViniciusDev26/prunr/commit/133be318f27d7428e6702b293075016192e34b36))
* create a new npm package by hand, without a script ([#13](https://github.com/ViniciusDev26/prunr/issues/13)) ([4827b1a](https://github.com/ViniciusDev26/prunr/commit/4827b1abb0ea4faaa201f3d779fda4a09dbd19b7))
* initial README, and point 1 of the MVP done ([9d6bd1e](https://github.com/ViniciusDev26/prunr/commit/9d6bd1e96e8388d7cbee839fa410232eb0a6a275))
* mvp definition and tdd rules ([f6df851](https://github.com/ViniciusDev26/prunr/commit/f6df851b3f1626f99d85da9b1652110162ba0e75))
