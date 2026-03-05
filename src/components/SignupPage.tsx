import { FormEvent, useState } from "react";
import { motion } from "framer-motion";

interface SignupPageProps {
  onSignup: (email: string, password: string) => Promise<void>;
  onSwitchToLogin: () => void;
}

export function SignupPage({ onSignup, onSwitchToLogin }: SignupPageProps) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setError("");
    setIsSubmitting(true);
    try {
      await onSignup(email.trim(), password);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div style={{ minHeight: "100vh", background: "radial-gradient(circle at top right, #2a1846 0%, #0d0d0d 55%)", display: "grid", placeItems: "center", padding: "24px" }}>
      <motion.form
        onSubmit={submit}
        initial={{ opacity: 0, y: 24 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease: "easeOut" }}
        style={{ width: "100%", maxWidth: "440px", background: "#161616", border: "1px solid #262626", borderRadius: "16px", padding: "32px", boxShadow: "0 24px 64px rgba(0,0,0,0.45)" }}
      >
        <h1 style={{ color: "#eaeaea", margin: 0, fontSize: "32px", fontWeight: 700 }}>Create Account</h1>
        <p style={{ color: "#a7a7a7", marginTop: "8px", marginBottom: "28px" }}>Secure local account with encrypted credentials</p>

        <label style={{ display: "block", color: "#d6d6d6", fontSize: "13px", marginBottom: "8px" }}>Email</label>
        <input
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          required
          style={{ width: "100%", marginBottom: "14px", background: "#101010", border: "1px solid #2e2e2e", color: "#f1f1f1", borderRadius: "10px", padding: "12px 14px" }}
        />

        <label style={{ display: "block", color: "#d6d6d6", fontSize: "13px", marginBottom: "8px" }}>Password (min 8 chars)</label>
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          required
          minLength={8}
          style={{ width: "100%", marginBottom: "18px", background: "#101010", border: "1px solid #2e2e2e", color: "#f1f1f1", borderRadius: "10px", padding: "12px 14px" }}
        />

        {error && <p style={{ color: "#ff6f91", marginTop: 0, marginBottom: "12px" }}>{error}</p>}

        <motion.button
          type="submit"
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.98 }}
          disabled={isSubmitting}
          style={{ width: "100%", border: "none", borderRadius: "10px", padding: "12px 14px", background: "#6c5ce7", color: "#ffffff", fontWeight: 700, cursor: "pointer" }}
        >
          {isSubmitting ? "Creating..." : "Sign Up"}
        </motion.button>

        <button type="button" onClick={onSwitchToLogin} style={{ marginTop: "14px", background: "transparent", border: "none", color: "#b8b8b8", cursor: "pointer" }}>
          Already have an account? Login
        </button>
      </motion.form>
    </div>
  );
}
