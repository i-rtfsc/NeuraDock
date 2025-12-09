/**
 * 前端日志工具
 *
 * 将前端日志上报到后端，统一记录到日志文件中
 */

import { invoke } from '@tauri-apps/api/core';

type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error';

interface LogFields {
  [key: string]: any;
}

class Logger {
  private defaultTarget = 'frontend::app';

  /**
   * 记录日志并上报到后端
   */
  private async log(
    level: LogLevel,
    target: string,
    message: string,
    fields?: LogFields
  ) {
    try {
      // 上报到后端（fields 需要序列化为 JSON 字符串）
      await invoke('log_from_frontend', {
        level,
        target,
        message,
        fields: fields ? JSON.stringify(fields) : null
      });
    } catch (error) {
      // 降级到 console，但不抛出错误（避免日志记录阻塞业务）
      console[level]?.(`[${target}] ${message}`, fields, error);
    }
  }

  /**
   * TRACE 级别日志（最详细）
   */
  trace(message: string, fields?: LogFields, target?: string) {
    this.log('trace', target || this.defaultTarget, message, fields);
  }

  /**
   * DEBUG 级别日志（调试信息）
   */
  debug(message: string, fields?: LogFields, target?: string) {
    this.log('debug', target || this.defaultTarget, message, fields);
  }

  /**
   * INFO 级别日志（一般信息）
   */
  info(message: string, fields?: LogFields, target?: string) {
    this.log('info', target || this.defaultTarget, message, fields);
  }

  /**
   * WARN 级别日志（警告）
   */
  warn(message: string, fields?: LogFields, target?: string) {
    this.log('warn', target || this.defaultTarget, message, fields);
  }

  /**
   * ERROR 级别日志（错误）
   */
  error(message: string, error?: Error | unknown, fields?: LogFields, target?: string) {
    const errorFields: LogFields = {
      ...fields,
    };

    // 解析 Error 对象
    if (error instanceof Error) {
      errorFields.error_name = error.name;
      errorFields.error_message = error.message;
      errorFields.error_stack = error.stack;
    } else if (error) {
      errorFields.error = String(error);
    }

    this.log('error', target || this.defaultTarget, message, errorFields);
  }

  /**
   * 记录用户操作
   */
  userAction(action: string, details?: LogFields) {
    this.info(`User action: ${action}`, details, 'frontend::user_action');
  }

  /**
   * 记录 API 请求
   */
  apiCall(
    method: string,
    url: string,
    status?: number,
    duration?: number,
    error?: Error
  ) {
    const fields: LogFields = {
      method,
      url,
      status,
      duration_ms: duration,
    };

    if (error) {
      this.error(`API call failed: ${method} ${url}`, error, fields, 'frontend::api');
    } else {
      this.info(`API call: ${method} ${url}`, fields, 'frontend::api');
    }
  }

  /**
   * 记录性能指标
   */
  performance(metric: string, value: number, unit: string = 'ms', fields?: LogFields) {
    this.info(`Performance: ${metric}`, {
      ...fields,
      metric,
      value,
      unit,
    }, 'frontend::performance');
  }
}

// 导出单例
export const logger = new Logger();

// 全局错误处理
if (typeof window !== 'undefined') {
  // 捕获未处理的 Promise rejection
  window.addEventListener('unhandledrejection', (event) => {
    logger.error(
      'Unhandled promise rejection',
      event.reason,
      {
        promise: event.promise?.toString(),
      },
      'frontend::global_error'
    );
  });

  // 捕获全局错误
  window.addEventListener('error', (event) => {
    logger.error(
      'Global error caught',
      event.error,
      {
        message: event.message,
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno,
      },
      'frontend::global_error'
    );
  });
}
