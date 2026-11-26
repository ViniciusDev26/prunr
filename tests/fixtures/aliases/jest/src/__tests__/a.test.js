const createRuntime = require("createRuntime");
const math = require("@lib/math");
require("./styles.css");
test("x", () => createRuntime() && math());
