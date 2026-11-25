const name = process.argv[2];

require(`./handlers/${name}`).run();
require("./jobs/" + name + ".job.js");
require(`./missing/${name}`);
require(`${name}/plugin`);
