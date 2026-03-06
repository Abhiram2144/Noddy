import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import { Send, Bot, User, Sparkles } from "lucide-react";

interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: Date;
}

export function ChatView() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputMessage, setInputMessage] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
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
      // Call Tauri backend
      const response = await invoke<string>("chat_with_ai", {
        message: trimmedMessage,
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
