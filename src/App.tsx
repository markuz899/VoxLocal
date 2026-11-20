import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

// ponytail: placeholder di Fase 2, impostazioni/cronologia/onboarding arrivano in Fase 4
export default function App() {
  const [state, setState] = useState("idle");

  useEffect(() => {
    const off = listen<{ state: string }>("pipeline://state", (e) =>
      setState(e.payload.state),
    );
    return () => {
      off.then((f) => f());
    };
  }, []);

  return (
    <main className="p-6 font-sans">
      <h1 className="text-xl font-semibold">VoxLocal</h1>
      <p className="mt-2 text-sm text-gray-500">Stato: {state}</p>
    </main>
  );
}
