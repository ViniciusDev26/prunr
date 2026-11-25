import { Test } from "@nestjs/testing";
import { AppModule } from "../src/app.module";

test("boots", async () => {
  await Test.createTestingModule({ imports: [AppModule] }).compile();
});
