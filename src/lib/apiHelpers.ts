import { toast } from "../components/Toast";

/**
 * Wraps an async API call with error handling and toast notifications.
 * 
 * @param apiCall - The async function to call
 * @param options - Configuration for the wrapper
 * @returns The result of the API call
 */
export async function withApiToast<T>(
  apiCall: () => Promise<T>,
  options: {
    /** Message to show on success */
    successMessage?: string;
    /** Message to show on error */
    errorMessage?: string;
    /** Whether to show success toast (default: false) */
    showSuccess?: boolean;
    /** Whether to show error toast (default: true) */
    showError?: boolean;
    /** Toast duration in ms */
    duration?: number;
  } = {}
): Promise<T> {
  const {
    successMessage,
    errorMessage,
    showSuccess = false,
    showError = true,
    duration = 5000,
  } = options;

  try {
    const result = await apiCall();
    
    if (showSuccess && successMessage) {
      toast("success", successMessage, duration);
    }
    
    return result;
  } catch (error) {
    const message = error instanceof Error ? error.message : errorMessage ?? "An unexpected error occurred";
    
    if (showError) {
      toast("error", message, duration);
    }
    
    // Re-throw to allow callers to handle if needed
    throw error;
  }
}

/**
 * Creates a wrapped version of an API function with default error handling.
 * Useful for creating consistent error handling across multiple calls.
 * 
 * @param apiFn - The API function to wrap
 * @param options - Default options for all calls to this function
 * @returns Wrapped function
 */
export function createApiWrapper<T extends (...args: unknown[]) => Promise<unknown>>(
  apiFn: T,
  options: {
    errorMessage?: string;
    showError?: boolean;
  } = {}
) {
  const { errorMessage, showError = true } = options;
  
  return async (...args: Parameters<T>): Promise<unknown> => {
    try {
      return await apiFn(...args);
    } catch (error) {
      const message = error instanceof Error ? error.message : errorMessage ?? "Operation failed";
      
      if (showError) {
        toast("error", message);
      }
      
      throw error;
    }
  };
}