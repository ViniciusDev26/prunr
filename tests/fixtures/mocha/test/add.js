const { expect } = require("chai");
const { add } = require("../lib");
it("adds", () => expect(add(1, 2)).to.equal(3));
