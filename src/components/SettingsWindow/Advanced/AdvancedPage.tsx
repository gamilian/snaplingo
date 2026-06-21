import { useState } from 'react';
import { CustomSelect } from '../../common/CustomSelect';
import { CustomNumberInput } from '../../common/CustomNumberInput';
import { triggerScreenshot } from '../../../tauri/captureSession';

export function AdvancedPage() {
  const [proxyMode, setProxyMode] = useState('none');
  const [retryCount, setRetryCount] = useState('3');
  const [logLevel, setLogLevel] = useState('error');
  const [timeout, setTimeout] = useState(10);
  const [historyDays, setHistoryDays] = useState(30);

  const handleTestScreenshot = async () => {
    try {
      console.log('Calling trigger_screenshot command...');
      await triggerScreenshot();
      console.log('Screenshot triggered successfully');
    } catch (error) {
      console.error('Failed to trigger screenshot:', error);
      alert(`触发截图失败: ${error}`);
    }
  };

  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">高级设置</h2>
        <p className="text-gray-600">面向高级用户的配置选项</p>
      </div>

      {/* 测试工具 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">测试工具</h3>

        <div>
          <button
            onClick={handleTestScreenshot}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
          >
            测试截图功能（后端事件）
          </button>
          <p className="text-sm text-gray-500 mt-2">
            通过后端触发截图事件，测试 Tauri 事件系统是否正常工作
          </p>
        </div>
      </div>

      {/* 网络设置 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">网络</h3>

        {/* 代理设置 */}
        <div>
          <label className="block font-medium text-gray-700 mb-2">代理设置</label>
          <CustomSelect
            options={[
              { value: 'none', label: '不使用代理' },
              { value: 'system', label: '使用系统代理' },
              { value: 'custom', label: '自定义代理' },
            ]}
            value={proxyMode}
            onChange={setProxyMode}
          />

          {/* 自定义代理输入框（条件显示） */}
          {proxyMode === 'custom' && (
            <div className="mt-3 space-y-3 pl-4 border-l-2 border-gray-200">
              <input
                type="text"
                placeholder="http://127.0.0.1:7890"
                className="w-full px-4 py-2.5 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent transition-all duration-150"
              />
              <p className="text-sm text-gray-500">格式：http://host:port 或 socks5://host:port</p>
            </div>
          )}
        </div>

        {/* 超时时间 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">请求超时时间（秒）</label>
          <CustomNumberInput
            value={timeout}
            onChange={setTimeout}
            min={5}
            max={60}
            step={1}
          />
          <p className="text-sm text-gray-500 mt-2">API 请求超时时间，默认 10 秒</p>
        </div>

        {/* 重试次数 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">失败重试次数</label>
          <CustomSelect
            options={[
              { value: '0', label: '不重试' },
              { value: '1', label: '1 次' },
              { value: '2', label: '2 次' },
              { value: '3', label: '3 次（推荐）' },
              { value: '5', label: '5 次' },
            ]}
            value={retryCount}
            onChange={setRetryCount}
          />
          <p className="text-sm text-gray-500 mt-2">网络请求失败时的重试次数</p>
        </div>
      </div>

      {/* 日志和调试 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">日志和调试</h3>

        {/* 日志级别 */}
        <div>
          <label className="block font-medium text-gray-700 mb-2">日志级别</label>
          <CustomSelect
            options={[
              { value: 'error', label: '错误（Error）' },
              { value: 'warn', label: '警告（Warn）' },
              { value: 'info', label: '信息（Info）' },
              { value: 'debug', label: '调试（Debug）' },
            ]}
            value={logLevel}
            onChange={setLogLevel}
          />
          <p className="text-sm text-gray-500 mt-2">日志级别越高记录的信息越详细</p>
        </div>

        {/* 日志保存 */}
        <div className="pt-6 border-t border-gray-100 flex items-center justify-between">
          <div>
            <div className="font-medium text-gray-700">保存日志到文件</div>
            <div className="text-sm text-gray-500 mt-1">将日志输出到文件，便于排查问题</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-primary-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>

        {/* 打开日志目录 */}
        <div className="pt-6 border-t border-gray-100">
          <button className="px-4 py-2 text-sm text-primary-600 hover:bg-blue-50 rounded-lg transition-colors">
            打开日志目录
          </button>
        </div>
      </div>

      {/* 数据管理 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">数据管理</h3>

        {/* 历史记录 */}
        <div className="flex items-center justify-between">
          <div>
            <div className="font-medium text-gray-700">自动清理历史记录</div>
            <div className="text-sm text-gray-500 mt-1">自动删除超过一定天数的历史记录</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-primary-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>

        {/* 保留天数 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">历史记录保留天数</label>
          <CustomNumberInput
            value={historyDays}
            onChange={setHistoryDays}
            min={1}
            max={365}
            step={1}
          />
          <p className="text-sm text-gray-500 mt-2">超过此天数的历史记录将被自动删除</p>
        </div>

        {/* 清除数据 */}
        <div className="pt-6 border-t border-gray-100 space-y-3">
          <button className="w-full px-4 py-2 text-sm text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-lg transition-colors text-left">
            清除所有历史记录
          </button>
          <button className="w-full px-4 py-2 text-sm text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-lg transition-colors text-left">
            清除所有缓存
          </button>
          <button className="w-full px-4 py-2 text-sm text-red-600 bg-red-50 hover:bg-red-100 rounded-lg transition-colors text-left">
            重置所有设置
          </button>
        </div>
      </div>

      {/* 实验性功能 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <div>
          <h3 className="text-lg font-semibold text-gray-800">实验性功能</h3>
          <p className="text-sm text-gray-500 mt-1">这些功能可能不稳定，请谨慎启用</p>
        </div>

        <div className="flex items-center justify-between">
          <div>
            <div className="font-medium text-gray-700">GPU 加速</div>
            <div className="text-sm text-gray-500 mt-1">使用 GPU 加速 OCR 识别（需要支持 CUDA）</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-gray-300 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform" />
          </button>
        </div>

        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">性能监控</div>
            <div className="text-sm text-gray-500 mt-1">显示 CPU、内存占用等性能指标</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-gray-300 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform" />
          </button>
        </div>
      </div>
    </div>
  );
}
