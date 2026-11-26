# Changelog

## [0.6.0](https://github.com/ViniciusDev26/prunr/compare/v0.5.0...v0.6.0) (2026-11-26)


### Features

* a github action that runs prunr check ([#91](https://github.com/ViniciusDev26/prunr/issues/91)) ([e894060](https://github.com/ViniciusDev26/prunr/commit/e89406061b13466ea89fe932b050ef3b8492afdd))
* astro conventions and react router plugin ([#80](https://github.com/ViniciusDev26/prunr/issues/80)) ([b1f35ad](https://github.com/ViniciusDev26/prunr/commit/b1f35adff63ea17a8147520d430a67223acf0110))
* compile vue, svelte, astro and mdx files to their scripts ([#75](https://github.com/ViniciusDev26/prunr/issues/75)) ([cf87ad4](https://github.com/ViniciusDev26/prunr/commit/cf87ad41446be88b537bfda1d65cc1d1a6f222df))
* declarative tool plugins for eslint, prettier, babel and commitlint ([#76](https://github.com/ViniciusDev26/prunr/issues/76)) ([0eb96ef](https://github.com/ViniciusDev26/prunr/commit/0eb96efc3ba179ccccd60ccd307fe01758222a35))
* init translates a knip config ([#93](https://github.com/ViniciusDev26/prunr/issues/93)) ([ba9cff0](https://github.com/ViniciusDev26/prunr/commit/ba9cff027b04a4273e66833c9e5be06030c944a5))
* packages a library loads by the names passed to it are used ([#95](https://github.com/ViniciusDev26/prunr/issues/95)) ([ab67260](https://github.com/ViniciusDev26/prunr/commit/ab6726011e329285678abd98ef5a945052bf48f5))
* report the esm exports of the same binding under two names ([#90](https://github.com/ViniciusDev26/prunr/issues/90)) ([6c6b016](https://github.com/ViniciusDev26/prunr/commit/6c6b0168c603743e530915b9e740437843e877d7))
* report the script binaries only an undeclared package provides ([#92](https://github.com/ViniciusDev26/prunr/issues/92)) ([998fec0](https://github.com/ViniciusDev26/prunr/commit/998fec057dca9a4558d3719b7ee6217e05923651))
* report the uncalled methods of exported classes in applications ([#89](https://github.com/ViniciusDev26/prunr/issues/89)) ([b04232e](https://github.com/ViniciusDev26/prunr/commit/b04232e80260f48f703b3bc157839eecfbba4a33))
* report the unused members of exported enums ([#88](https://github.com/ViniciusDev26/prunr/issues/88)) ([35a22dd](https://github.com/ViniciusDev26/prunr/commit/35a22dd3098441099311d394c62052cf5b15db84))
* sveltekit and nuxt plugins ([#79](https://github.com/ViniciusDev26/prunr/issues/79)) ([b99b5e5](https://github.com/ViniciusDev26/prunr/commit/b99b5e5e65d7e2e08be6942ac03b0e52be60f408))
* the paths written in a config are the files the tool loads ([#94](https://github.com/ViniciusDev26/prunr/issues/94)) ([92da45a](https://github.com/ViniciusDev26/prunr/commit/92da45a3aa7a38960340794e5968d0fa44f1da0a))
* tool plugins for storybook, cypress, husky, lint-staged, tailwind, prisma, tsup, rollup and webpack ([#78](https://github.com/ViniciusDev26/prunr/issues/78)) ([e2298dc](https://github.com/ViniciusDev26/prunr/commit/e2298dc213e1b65109facfdee2f610942fc63112))
* turn plugins off or on in prunr.json ([#96](https://github.com/ViniciusDev26/prunr/issues/96)) ([db1d085](https://github.com/ViniciusDev26/prunr/commit/db1d0850816ce90e12f9ced12ee8cbb3dc4d31ab))
* typescript plugin reads tsconfig extends, types, plugins and references ([#73](https://github.com/ViniciusDev26/prunr/issues/73)) ([adc1748](https://github.com/ViniciusDev26/prunr/commit/adc17484a7442adbfb010cd058b52e4e9b18f03f))
* vitepress plugin ([#81](https://github.com/ViniciusDev26/prunr/issues/81)) ([97057de](https://github.com/ViniciusDev26/prunr/commit/97057defa8956a85d063f1a2b25cf73bdc8a4a34))


### Bug Fixes

* a root dependency that the packages using it declare themselves is unused ([#86](https://github.com/ViniciusDev26/prunr/issues/86)) ([77ffe5b](https://github.com/ViniciusDev26/prunr/commit/77ffe5b16bab78f3a331381b6993eb0139ddc039))
* compile single-file components per package and skip svelte prop exports ([#77](https://github.com/ViniciusDev26/prunr/issues/77)) ([dc859a5](https://github.com/ViniciusDev26/prunr/commit/dc859a5ca9a6c9f0e3cd78e56e5d2d6728d68ae5))
* **mocha:** skip the values of the watch, parallel and node flags ([#84](https://github.com/ViniciusDev26/prunr/issues/84)) ([fc99f25](https://github.com/ViniciusDev26/prunr/commit/fc99f25b6e497e2bad73c23d651bc6036912e2f7))
* **why:** follow reexports and explain entries, own-file uses and [@types](https://github.com/types) ([#85](https://github.com/ViniciusDev26/prunr/issues/85)) ([fbc8d14](https://github.com/ViniciusDev26/prunr/commit/fbc8d147def4dfe001c15fc14e35cfd2ad291a66))


### Documentation

* close phase 3 in the roadmap and refresh the benchmark ([#97](https://github.com/ViniciusDev26/prunr/issues/97)) ([44c8b09](https://github.com/ViniciusDev26/prunr/commit/44c8b0993221a79f528abc0eb036e761fcb41df0))
* plan phase 3 in the roadmap ([#83](https://github.com/ViniciusDev26/prunr/issues/83)) ([507668f](https://github.com/ViniciusDev26/prunr/commit/507668f67bf3aadf1a65684bee709787065e15e1))
* triage what only knip reports ([#87](https://github.com/ViniciusDev26/prunr/issues/87)) ([fee5b31](https://github.com/ViniciusDev26/prunr/commit/fee5b31c26e0e658f61159a115e0c129fcec9fed))
* update the knip agreement after phase 2 ([#82](https://github.com/ViniciusDev26/prunr/issues/82)) ([1830c81](https://github.com/ViniciusDev26/prunr/commit/1830c813e15cc56bc0d90673dba508398a789745))

## [0.5.0](https://github.com/ViniciusDev26/prunr/compare/v0.4.0...v0.5.0) (2026-11-26)


### Features

* jest's moduleNameMapper and __mocks__ and vite's resolve.alias resolve their names ([#70](https://github.com/ViniciusDev26/prunr/issues/70)) ([3a9aa78](https://github.com/ViniciusDev26/prunr/commit/3a9aa781d7075f23af68e2bc4ea50ce2098d2e2e))
* mocha, node --test, tap, ava, lab and borp test runners ([#56](https://github.com/ViniciusDev26/prunr/issues/56)) ([c89c586](https://github.com/ViniciusDev26/prunr/commit/c89c586bcb2725a23607a1340869c97acb51a6a9))
* prunr check --production reports only what goes to production ([#53](https://github.com/ViniciusDev26/prunr/issues/53)) ([1b89ea2](https://github.com/ViniciusDev26/prunr/commit/1b89ea285670bacc22d03f670273b0b2a90cc06a))
* prunr why explains why a file, an export or a package is alive ([#50](https://github.com/ViniciusDev26/prunr/issues/50)) ([893752a](https://github.com/ViniciusDev26/prunr/commit/893752a393bf3f2df69970e5a6a061ea69b2e6da))
* sarif, code climate and markdown reports ([#52](https://github.com/ViniciusDev26/prunr/issues/52)) ([076b638](https://github.com/ViniciusDev26/prunr/commit/076b63800447012a9e4533a2c448267c3e91700a))


### Bug Fixes

* a file that does not parse no longer stops the check ([#46](https://github.com/ViniciusDev26/prunr/issues/46)) ([768ec8d](https://github.com/ViniciusDev26/prunr/commit/768ec8d151e7c42ba99ccf2ebea90483d501a76b))
* a package any package.json above the file declares is not missing ([#49](https://github.com/ViniciusDev26/prunr/issues/49)) ([fb5d63c](https://github.com/ViniciusDev26/prunr/commit/fb5d63cb6d4832964814da3fd3bac94c36dfa2f8))
* a tsconfig that cannot be loaded no longer leaves every import unresolved ([#48](https://github.com/ViniciusDev26/prunr/issues/48)) ([141fb7a](https://github.com/ViniciusDev26/prunr/commit/141fb7a3041639acd3495e3c52e930e3f1cd0cb2))
* an import() in a jsdoc code example is prose, not an import ([#69](https://github.com/ViniciusDev26/prunr/issues/69)) ([16371c1](https://github.com/ViniciusDev26/prunr/commit/16371c10c1b54cfbe4b22f0beea664c18c653a1c))
* bin entries, the exported value passed to a function, and tracked files an ignore pattern matches ([#63](https://github.com/ViniciusDev26/prunr/issues/63)) ([0ca9eba](https://github.com/ViniciusDev26/prunr/commit/0ca9eba4e0f036a77d661518b71d15fdf469bbb0))
* every entry a published package declares is an entry, found in its source ([#60](https://github.com/ViniciusDev26/prunr/issues/60)) ([5c586d1](https://github.com/ViniciusDev26/prunr/commit/5c586d1ae3823a43e60916da57bd091d0ddd6fbb))
* false positives from a 55-project corpus ([#59](https://github.com/ViniciusDev26/prunr/issues/59)) ([451a314](https://github.com/ViniciusDev26/prunr/commit/451a314a3fa7475f985c89a4677b5814663c5218))
* find the entry exports declares, in the source a build compiles it from ([#51](https://github.com/ViniciusDev26/prunr/issues/51)) ([67be7c8](https://github.com/ViniciusDev26/prunr/commit/67be7c8953d0055b69491705fee7cb13812ffaf1))
* follow arguments a script forwards to another, and a tool run through its own bin ([#65](https://github.com/ViniciusDev26/prunr/issues/65)) ([76a3c56](https://github.com/ViniciusDev26/prunr/commit/76a3c563f98c5bddc02e86751ee8c2adbb9b1cca))
* read NODE_OPTIONS preloads, and let a package import itself by its name ([#54](https://github.com/ViniciusDev26/prunr/issues/54)) ([95e5a34](https://github.com/ViniciusDev26/prunr/commit/95e5a34d97c871a5699a8c8a94c22407c4cb8c3e))
* read package.json with a bom and fields of unexpected types ([#47](https://github.com/ViniciusDev26/prunr/issues/47)) ([ea068d1](https://github.com/ViniciusDev26/prunr/commit/ea068d1ff61e7d0bda38f2dd9c7d4c557fd9fc4c))
* read vitest projects written inline, and their test.dir ([#57](https://github.com/ViniciusDev26/prunr/issues/57)) ([486db1f](https://github.com/ViniciusDev26/prunr/commit/486db1f7f9748c2a0f199b12741fecdc8e879245))
* the exports of a typescript namespace are its members, not the module's ([#68](https://github.com/ViniciusDev26/prunr/issues/68)) ([2ad5d00](https://github.com/ViniciusDev26/prunr/commit/2ad5d00634c03a1b19c13c6ec704d21a2213cf72))
* the root package can import the packages of the repository undeclared ([#55](https://github.com/ViniciusDev26/prunr/issues/55)) ([0d93c78](https://github.com/ViniciusDev26/prunr/commit/0d93c78ea89ffb98b01909f1e311435e4051dc81))
* the unresolved and missing findings the corpus triage showed ([#67](https://github.com/ViniciusDev26/prunr/issues/67)) ([f12d741](https://github.com/ViniciusDev26/prunr/commit/f12d7414651b58c6b8b6d8d7a6d319a52394d1ed))


### Performance

* analyze and link the files in parallel ([#41](https://github.com/ViniciusDev26/prunr/issues/41)) ([0e1a441](https://github.com/ViniciusDev26/prunr/commit/0e1a4418a82a43bb0dcbfb3c09f17a2d2542f0a9))


### Documentation

* add the roadmap items a large monorepo showed ([#42](https://github.com/ViniciusDev26/prunr/issues/42)) ([eb69285](https://github.com/ViniciusDev26/prunr/commit/eb69285a781046fbdf0f5f20d202f857dcdd794c))
* english readme, and the case for prunr against knip ([#44](https://github.com/ViniciusDev26/prunr/issues/44)) ([121fd63](https://github.com/ViniciusDev26/prunr/commit/121fd6332d377b72f0b91310de3d7b78f301d63c))
* record phase 1 and the benchmark against knip ([#58](https://github.com/ViniciusDev26/prunr/issues/58)) ([1fabc46](https://github.com/ViniciusDev26/prunr/commit/1fabc466d3f4bc34c34a91a653a826b792fb9511))
* record the corpus numbers on the final phase 1 main ([#66](https://github.com/ViniciusDev26/prunr/issues/66)) ([4b0fbe2](https://github.com/ViniciusDev26/prunr/commit/4b0fbe26f15cd5316707288f39d416b8b9994a4d))
* record what the does-not-exist findings of the 100-project corpus are ([#72](https://github.com/ViniciusDev26/prunr/issues/72)) ([02dc8ac](https://github.com/ViniciusDev26/prunr/commit/02dc8ac461af861917c14751726de2357fe24a7c))
* record why knip reports webpack's declaration types and prunr does not ([#64](https://github.com/ViniciusDev26/prunr/issues/64)) ([62730b3](https://github.com/ViniciusDev26/prunr/commit/62730b312f92f534ecaa88c113a0d6874e5a8027))
* the benchmark and the triage of the 55-project corpus ([#62](https://github.com/ViniciusDev26/prunr/issues/62)) ([d2fb629](https://github.com/ViniciusDev26/prunr/commit/d2fb629809b81d50ac1b7c5ce4538547b0ba6e96))

## [0.4.0](https://github.com/ViniciusDev26/prunr/compare/v0.3.0...v0.4.0) (2026-11-25)


### Features

* a package a used package needs as a peer is used ([#38](https://github.com/ViniciusDev26/prunr/issues/38)) ([94bb118](https://github.com/ViniciusDev26/prunr/commit/94bb1189897b9a2d0e082dfbb77b44565d429942))
* playwright, drizzle kit and postcss plugins, and vitest projects ([#39](https://github.com/ViniciusDev26/prunr/issues/39)) ([e984ef6](https://github.com/ViniciusDev26/prunr/commit/e984ef67de77d2a5528438ef05874d4efc5c4d83))
* the files the scripts run are entries ([#37](https://github.com/ViniciusDev26/prunr/issues/37)) ([3e5765a](https://github.com/ViniciusDev26/prunr/commit/3e5765a8e8b342b35d58fd50c4eda18c25035a57))
* the stylesheets a file imports use packages ([#40](https://github.com/ViniciusDev26/prunr/issues/40)) ([1fc6eda](https://github.com/ViniciusDev26/prunr/commit/1fc6edaa4528b0443f544e77eaaa591512caa792))
* vite and react plugins, with absolute imports from the public dir ([#36](https://github.com/ViniciusDev26/prunr/issues/36)) ([0a65c48](https://github.com/ViniciusDev26/prunr/commit/0a65c48cbf1fed10b44e1b38e64860aa5e4bed58))


### Documentation

* rewrite the readme and move the reference into docs/guide ([#34](https://github.com/ViniciusDev26/prunr/issues/34)) ([d84b0ac](https://github.com/ViniciusDev26/prunr/commit/d84b0ac6026b86d554f009d943a8810b0a9b8d55))

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
