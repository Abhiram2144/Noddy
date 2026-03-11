import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { motion, AnimatePresence } from "framer-motion";
import { Send, Bot, User, Sparkles } from "lucide-react";
import { useAuth } from "../auth/AuthContext";

interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: Date;
  reminderId?: string;
}

interface PersistedMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  created_at: number;
}

interface Suggestion {
  id: string;
  user_id: string;
  message: string;
  action_intent?: string;
  parameters?: Record<string, unknown>;
  priority: number;
  timestamp: number;
}

interface ActionResponse {
  success: boolean;
  message: string;
}

export function ChatView() {
  const { getAccessToken } = useAuth();
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputMessage, setInputMessage] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // Load persisted chat history on mount.
  useEffect(() => {
    const loadHistory = async () => {
      try {
        const accessToken = await getAccessToken();
        const history = await invoke<PersistedMessage[]>("get_chat_history", {
          accessToken,
          limit: 200,
        });

        if (Array.isArray(history)) {
          setMessages(
            history.map((entry) => ({
              id: entry.id,
              role: entry.role,
              content: entry.content,
              timestamp: new Date((entry.created_at || 0) * 1000),
            }))
          );
        }
      } catch (error) {
        console.error("Failed to load chat history:", error);
      }
    };

    void loadHistory();
  }, [getAccessToken]);

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Inject reminder alerts into chat when they fire
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<{ id: string; content: string; user_id: string }>(
      "reminder_fired",
      (event) => {
        setMessages((prev) => [
          ...prev,
          {
            id: `reminder-${Date.now()}`,
            role: "assistant" as const,
            content: `⏰ Reminder: ${event.payload.content}`,
            timestamp: new Date(),
            reminderId: event.payload.id,
          },
        ]);
      }
    ).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  // Listen for proactive suggestion events.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<Suggestion>("suggestion_generated", (event) => {
      setSuggestions((prev) => {
        const next = [event.payload, ...prev.filter((s) => s.id !== event.payload.id)];
        next.sort((a, b) => b.priority - a.priority);
        return next.slice(0, 4);
      });
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  const handleSendMessage = async () => {
    const trimmedMessage = inputMessage.trim();
    if (!trimmedMessage || isLoading) return;

    // Create user message
    const userMessage: Message = {
      id: `user-${Date.now()}`,
      role: "user",
      content: trimmedMessage,
      timestamp: new Date(),
    };

    // Add user message to chat
    setMessages((prev) => [...prev, userMessage]);
    setInputMessage("");
    setIsLoading(true);

    try {
      const accessToken = await getAccessToken();

      // Call Tauri backend
      const response = await invoke<string>("chat_with_ai", {
        message: trimmedMessage,
        accessToken,
      });

      // Create assistant message
      const assistantMessage: Message = {
        id: `assistant-${Date.now()}`,
        role: "assistant",
        content: response,
        timestamp: new Date(),
      };

      // Add assistant message to chat
      setMessages((prev) => [...prev, assistantMessage]);
    } catch (error) {
      console.error("Error sending message:", error);
      
      // Create error message
      const errorMessage: Message = {
        id: `error-${Date.now()}`,
        role: "assistant",
        content: `Error: ${error}`,
        timestamp: new Date(),
      };

      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      setIsLoading(false);
      inputRef.current?.focus();
    }
  };

  const handleReminderDone = async (reminderId: string) => {
    try {
      const accessToken = await getAccessToken();
      await invoke<string>("finish_reminder", { accessToken, reminderId });
      setMessages((prev) =>
        prev.map((message) =>
          message.reminderId === reminderId
            ? { ...message, content: `${message.content}\nDone.` }
            : message
        )
      );
    } catch (error) {
      console.error("Failed to finish reminder:", error);
    }
  };

  const handleReminderSnooze = async (reminderId: string, snoozeMinutes: number) => {
    try {
      const accessToken = await getAccessToken();
      await invoke<string>("snooze_reminder", { accessToken, reminderId, snoozeMinutes });
      setMessages((prev) =>
        prev.map((message) =>
          message.reminderId === reminderId
            ? { ...message, content: `${message.content}\nSnoozed ${snoozeMinutes} min.` }
            : message
        )
      );
    } catch (error) {
      console.error("Failed to snooze reminder:", error);
    }
  };

  const handleExecuteSuggestion = async (suggestion: Suggestion) => {
    if (!suggestion.action_intent) {
      return;
    }

    try {
      const accessToken = await getAccessToken();
      const intentJson = JSON.stringify({
        name: suggestion.action_intent,
        payload: suggestion.parameters ?? {},
      });

      const result = await invoke<ActionResponse>("execute_action", {
        intentJson,
        accessToken,
      });

      const assistantMessage: Message = {
        id: `suggestion-exec-${Date.now()}`,
        role: "assistant",
        content: result.message,
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, assistantMessage]);
      setSuggestions((prev) => prev.filter((s) => s.id !== suggestion.id));
    } catch (error) {
      console.error("Failed to execute suggestion:", error);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  return (
    <motion.div
      className="panel-container"
      initial={{ opacity: 0, x: 40 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -40 }}
      transition={{ duration: 0.4 }}
      style={{ display: "flex", flexDirection: "column", height: "100%" }}
    >
      {/* Header */}
      <div className="panel-header">
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <h1 className="panel-title">AI Chat</h1>
            <p className="panel-subtitle">Chat with Noddy AI powered by Gemini 1.5 Flash</p>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "8px", color: "var(--text-secondary)", fontSize: "13px" }}>
            <Sparkles style={{ width: "16px", height: "16px", color: "var(--accent-primary)" }} />
            <span>{messages.length} messages</span>
          </div>
        </div>
      </div>

      {/* Messages Area */}
      <div style={{ flex: 1, overflowY: "auto", padding: "24px", display: "flex", flexDirection: "column", gap: "16px" }}>
        {suggestions.length > 0 && (
          <div
            style={{
              border: "1px solid var(--border-medium)",
              borderRadius: "12px",
              background: "var(--bg-elevated)",
              padding: "12px",
              display: "flex",
              flexDirection: "column",
              gap: "8px",
            }}
          >
            <p style={{ margin: 0, fontSize: "12px", color: "var(--text-secondary)", fontWeight: 600 }}>
              Suggestions
            </p>
            {suggestions.map((s) => (
              <div
                key={s.id}
                style={{
                  border: "1px solid var(--border-subtle)",
                  borderRadius: "10px",
                  padding: "10px",
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  gap: "10px",
                }}
              >
                <span style={{ fontSize: "13px", color: "var(--text-primary)" }}>{s.message}</span>
                {s.action_intent && (
                  <button
                    onClick={() => handleExecuteSuggestion(s)}
                    className="btn btn-primary"
                    style={{ padding: "6px 10px", fontSize: "12px" }}
                  >
                    Do it
                  </button>
                )}
              </div>
            ))}
          </div>
        )}

        {messages.length === 0 && (
          <motion.div
            className="empty-state"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4 }}
          >
            <Bot style={{ width: "48px", height: "48px", color: "var(--text-tertiary)", marginBottom: "16px" }} />
            <p style={{ fontSize: "18px", fontWeight: 600, color: "var(--text-secondary)", marginBottom: "8px" }}>
              Start a conversation with Noddy AI
            </p>
            <p style={{ fontSize: "14px", color: "var(--text-tertiary)" }}>
              Type a message below to get started
            </p>
          </motion.div>
        )}

        <AnimatePresence>
          {messages.map((message, index) => (
            <motion.div
              key={message.id}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.2, delay: index * 0.05 }}
              style={{
                display: "flex",
                gap: "12px",
                justifyContent: message.role === "user" ? "flex-end" : "flex-start",
              }}
            >
              {message.role === "assistant" && (
                <div style={{ 
                  width: "32px", 
                  height: "32px", 
                  borderRadius: "8px", 
                  background: "var(--bg-elevated)", 
                  border: "1px solid var(--border-medium)",
                  display: "flex", 
                  alignItems: "center", 
                  justifyContent: "center",
                  flexShrink: 0,
                }}>
                  <Bot style={{ width: "18px", height: "18px", color: "var(--accent-primary)" }} />
                </div>
              )}

              <div style={{
                maxWidth: "70%",
                padding: "12px 16px",
                borderRadius: "12px",
                background: message.role === "user" ? "var(--accent-primary)" : "var(--bg-elevated)",
                border: message.role === "user" ? "none" : "1px solid var(--border-medium)",
                color: message.role === "user" ? "#ffffff" : "var(--text-primary)",
              }}>
                <p style={{ 
                  fontSize: "14px", 
                  lineHeight: "1.6", 
                  whiteSpace: "pre-wrap", 
                  wordBreak: "break-word",
                  margin: 0,
                }}>
                  {message.content}
                </p>
                <p style={{ 
                  fontSize: "11px", 
                  marginTop: "8px", 
                  opacity: 0.6,
                  margin: "8px 0 0 0",
                }}>
                  {message.timestamp.toLocaleTimeString([], {
                    hour: "2-digit",
                    minute: "2-digit",
                  })}
                </p>
                {message.reminderId && (
                  <div style={{ display: "flex", gap: "8px", marginTop: "10px" }}>
                    <button
                      onClick={() => handleReminderSnooze(message.reminderId!, 10)}
                      style={{
                        border: "1px solid var(--border-medium)",
                        borderRadius: "8px",
                        padding: "4px 10px",
                        fontSize: "12px",
                        background: "transparent",
                        color: "var(--text-primary)",
                        cursor: "pointer",
                      }}
                    >
                      Snooze 10m
                    </button>
                    <button
                      onClick={() => handleReminderDone(message.reminderId!)}
                      style={{
                        border: "1px solid var(--border-medium)",
                        borderRadius: "8px",
                        padding: "4px 10px",
                        fontSize: "12px",
                        background: "transparent",
                        color: "var(--text-primary)",
                        cursor: "pointer",
                      }}
                    >
                      Done
                    </button>
                  </div>
                )}
              </div>

              {message.role === "user" && (
                <div style={{ 
                  width: "32px", 
                  height: "32px", 
                  borderRadius: "8px", 
                  background: "var(--accent-primary)", 
                  display: "flex", 
                  alignItems: "center", 
                  justifyContent: "center",
                  flexShrink: 0,
                }}>
                  <User style={{ width: "18px", height: "18px", color: "#ffffff" }} />
                </div>
              )}
            </motion.div>
          ))}
        </AnimatePresence>

        {isLoading && (
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            style={{ display: "flex", gap: "12px", justifyContent: "flex-start" }}
          >
            <div style={{ 
              width: "32px", 
              height: "32px", 
              borderRadius: "8px", 
              background: "var(--bg-elevated)", 
              border: "1px solid var(--border-medium)",
              display: "flex", 
              alignItems: "center", 
              justifyContent: "center",
              flexShrink: 0,
            }}>
              <Bot style={{ width: "18px", height: "18px", color: "var(--accent-primary)" }} />
            </div>
            <div style={{
              padding: "12px 16px",
              borderRadius: "12px",
              background: "var(--bg-elevated)",
              border: "1px solid var(--border-medium)",
            }}>
              <div style={{ display: "flex", gap: "4px", alignItems: "center" }}>
                <span style={{ width: "6px", height: "6px", background: "var(--accent-primary)", borderRadius: "50%", animation: "bounce 1.4s infinite ease-in-out both", animationDelay: "-0.32s" }} />
                <span style={{ width: "6px", height: "6px", background: "var(--accent-primary)", borderRadius: "50%", animation: "bounce 1.4s infinite ease-in-out both", animationDelay: "-0.16s" }} />
                <span style={{ width: "6px", height: "6px", background: "var(--accent-primary)", borderRadius: "50%", animation: "bounce 1.4s infinite ease-in-out both" }} />
              </div>
            </div>
          </motion.div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input Area */}
      <div style={{ 
        padding: "20px 24px", 
        borderTop: "1px solid var(--border-subtle)",
        background: "var(--bg-secondary)",
      }}>
        <div style={{ display: "flex", gap: "12px" }}>
          <input
            ref={inputRef}
            type="text"
            value={inputMessage}
            onChange={(e) => setInputMessage(e.target.value)}
            onKeyPress={handleKeyPress}
            placeholder="Type your message..."
            disabled={isLoading}
            className="search-input"
            style={{
              flex: 1,
              padding: "12px 16px",
              fontSize: "14px",
            }}
          />
          <motion.button
            onClick={handleSendMessage}
            disabled={!inputMessage.trim() || isLoading}
            className="btn btn-primary"
            style={{
              display: "flex",
              alignItems: "center",
              gap: "8px",
              padding: "12px 24px",
            }}
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
          >
            <Send style={{ width: "16px", height: "16px" }} />
            Send
          </motion.button>
        </div>
      </div>
    </motion.div>
  );
}
