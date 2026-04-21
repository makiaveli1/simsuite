import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  /** Called with the error and the component stack trace when a child throws. */
  onError?: (error: Error, info: { componentStack: string }) => void;
  /** Content to show when a child throws. */
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

/**
 * Catches React render errors from children and renders a fallback instead.
 * This prevents entire trees from going blank when a leaf component throws.
 */
export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: { componentStack: string }) {
    console.error("[ErrorBoundary] caught render error:", error, info.componentStack);
    this.props.onError?.(error, info);
  }

  override render(): ReactNode {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback;
      return (
        <div style={{
          padding: "2rem",
          color: "var(--text-dim, #7f928a)",
          fontSize: "0.85rem",
          textAlign: "center",
        }}>
          Something went wrong loading details.
        </div>
      );
    }
    return this.props.children;
  }
}
