import { Test } from "@nestjs/testing";
import { AppController } from "./app.controller";
import { AppService } from "./app.service";

test("hello", async () => {
  const module = await Test.createTestingModule({
    controllers: [AppController],
    providers: [AppService],
  }).compile();
  expect(module.get(AppController).getHello()).toBe("Hello");
});
