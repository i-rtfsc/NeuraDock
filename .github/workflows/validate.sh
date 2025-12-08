#!/bin/bash
set -e

echo "🔍 验证 GitHub Actions Workflow..."
echo ""

# 检查必要的工具
check_tool() {
    if ! command -v $1 &> /dev/null; then
        echo "❌ $1 未安装"
        return 1
    else
        echo "✅ $1 已安装"
        return 0
    fi
}

echo "📋 检查依赖工具："
ALL_OK=true

# actionlint - GitHub Actions 语法检查
if ! check_tool actionlint; then
    echo "   安装: brew install actionlint"
    ALL_OK=false
fi

# yamllint - YAML 语法检查
if ! check_tool yamllint; then
    echo "   安装: brew install yamllint"
    ALL_OK=false
fi

echo ""

if [ "$ALL_OK" = false ]; then
    echo "⚠️  缺少必要工具，请先安装"
    echo ""
    echo "快速安装所有工具："
    echo "  brew install actionlint yamllint"
    echo ""
    exit 1
fi

echo "🧪 运行验证..."
echo ""

# 验证 YAML 语法
echo "1️⃣  检查 YAML 语法..."
yamllint .github/workflows/*.yml || {
    echo "❌ YAML 语法错误"
    exit 1
}
echo "✅ YAML 语法正确"
echo ""

# 验证 GitHub Actions 语法
echo "2️⃣  检查 GitHub Actions 语法..."
actionlint .github/workflows/*.yml || {
    echo "❌ GitHub Actions 配置错误"
    exit 1
}
echo "✅ GitHub Actions 配置正确"
echo ""

echo "✅ 所有检查通过！"
echo ""
echo "📝 下一步："
echo "  1. 提交到测试分支: git checkout -b test-ci"
echo "  2. 推送: git push origin test-ci"
echo "  3. 在 GitHub 手动触发 'Test Build' workflow"
echo "  4. 验证成功后合并到 main"
