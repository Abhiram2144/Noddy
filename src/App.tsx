import { invoke } from "@tauri-apps/api/core";

function App() {
  return (
    <div style={{ padding: "40px" }}>
      <h1>Noddy 🧠</h1>

      <button onClick={() => invoke("open_app", { app: "code" })}>
        Open VSCode
      </button>

      <br /><br />

      <button onClick={() => invoke("open_url", { url: "https://youtube.com" })}>
        Open YouTube
      </button>
    </div>
  );
}

export default App;