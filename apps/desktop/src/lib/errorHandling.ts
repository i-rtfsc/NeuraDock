/**
 * Common error handling utilities
 */

/** Unified error type for API errors */
export interface ApiError {
  message: string;
  code?: string;
  details?: unknown;
}

/** Type guard to check if an error is an ApiError */
export function isApiError(error: unknown): error is ApiError {
  return (
    typeof error === 'object' &&
    error !== null &&
    'message' in error &&
    typeof (error as ApiError).message === 'string'
  );
}

/** Extract error message from various error types */
export function extractErrorMessage(
  error: unknown,
  fallback = 'Unknown error'
): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (isApiError(error)) {
    return error.message;
  }
  if (typeof error === 'string') {
    return error;
  }
  return fallback;
}

/** Convert unknown error to ApiError */
export function toApiError(error: unknown): ApiError {
  if (isApiError(error)) {
    return error;
  }
  if (error instanceof Error) {
    return {
      message: error.message,
      details: error,
    };
  }
  if (typeof error === 'string') {
    return {
      message: error,
    };
  }
  return {
    message: 'Unknown error',
    details: error,
  };
}
