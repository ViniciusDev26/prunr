const request = require("supertest");
const { used } = require("./index");

test("used", () => expect(used()).toBe(1));
request;
