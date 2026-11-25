import reactLogo from "./assets/react.svg";
import viteLogo from "/vite.svg";
import { Button } from "./components/Button";

export default function App() {
  return (
    <>
      <img src={viteLogo} alt="Vite" />
      <img src={reactLogo} alt="React" />
      <Button label="count" />
    </>
  );
}
