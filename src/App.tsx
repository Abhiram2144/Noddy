import { invoke } from "@tauri-apps/api/core";

function App() {
  const handleAction = async (action: string, value: string) => {
    try {
      const result = await invoke<string>("execute_action", { action, value });
      console.log(result);
    } catch (error) {
      console.error("Error:", error);
    }
  };

  return (
    <div style={{ padding: "40px" }}>
      <h1>Noddy 🧠</h1>

      <button onClick={() => handleAction("open_app", "code")}>
        Open VSCode
      </button>

      <br /><br />

      <button onClick={() => handleAction("open_url", "https://youtube.com")}>
        Open YouTube
      </button>
    </div>
  );
}

export default App;