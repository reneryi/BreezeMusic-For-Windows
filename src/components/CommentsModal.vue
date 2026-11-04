<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useAppStore } from "../stores/appStore";
import { kugouClient } from "../api/kugou";
import type { SongComment } from "../types";
import PlumpIcon from "./PlumpIcon.vue";

const store = useAppStore();

const targetSong = computed(() => store.commentsModalSong || store.currentSong);

// 弹窗二级视图：主评论流 vs 楼中楼树状详情 vs 查看对话弹窗
const activeFloorRoot = ref<SongComment | null>(null);
const floorReplies = ref<EnrichedReply[]>([]);
const floorPage = ref(1);
const floorHasMore = ref(true);
const isLoadingFloor = ref(false);

const mainPage = ref(1);
const mainHasMore = ref(true);
const isLoadingMoreMain = ref(false);

const conversationChain = ref<SongComment[] | null>(null);
const previewImageUrl = ref<string | null>(null);

// 评论输入与表情面板
const commentInput = ref("");
const replyTarget = ref<SongComment | null>(null);
const showEmojiPicker = ref(false);
const isSubmitting = ref(false);

const emojiList = [
  "😀", "😂", "🥹", "😍", "🤩", "😭", "🥺", "🥳",
  "😎", "🤔", "🙌", "👍", "👏", "❤️", "🔥", "✨",
  "🎵", "🎶", "🎧", "🎸", "🎹", "🥁", "🌙", "🌊",
  "🌸", "🍻", "💯", "🙏", "💔", "🫡", "🫶", "🎉"
];

// 7 级炫彩树状嵌套配色（对齐 Android CommentsScreen.kt）
const clusterColors = [
  "#38BDF8", // Level 1 冰川蓝
  "#A855F7", // Level 2 极光紫
  "#EC4899", // Level 3 霓虹粉
  "#F59E0B", // Level 4 琥珀金
  "#10B981", // Level 5 翡翠绿
  "#6366F1", // Level 6 靛青蓝
  "#FB7185", // Level 7 珊瑚红
];

function getClusterColor(level: number): string {
  const idx = Math.max(0, (level - 1) % clusterColors.length);
  return clusterColors[idx];
}

watch(
  () => store.isCommentsModalOpen,
  (open) => {
    if (open) {
      activeFloorRoot.value = null;
      conversationChain.value = null;
      replyTarget.value = null;
      mainPage.value = 1;
      mainHasMore.value = true;
    }
  }
);

async function switchSort(sort: "hot" | "new") {
  if (!targetSong.value) return;
  mainPage.value = 1;
  mainHasMore.value = true;
  await store.loadCommentsForSong(targetSong.value, sort);
}

async function loadMoreMainComments() {
  if (!targetSong.value || isLoadingMoreMain.value || !mainHasMore.value) return;
  isLoadingMoreMain.value = true;
  try {
    const nextP = mainPage.value + 1;
    const more = await kugouClient.getSongComments(
      targetSong.value,
      store.commentSortType,
      nextP
    );
    if (more.length === 0) {
      mainHasMore.value = false;
    } else {
      const seen = new Set(store.activeSongComments.map((c) => c.id));
      const unique = more.filter((c) => !seen.has(c.id));
      store.activeSongComments.push(...unique);
      mainPage.value = nextP;
      if (more.length < 15 || unique.length === 0) {
        mainHasMore.value = false;
      }
    }
  } catch {
    mainHasMore.value = false;
  } finally {
    isLoadingMoreMain.value = false;
  }
}

async function openFloorDetail(rootCmt: SongComment) {
  activeFloorRoot.value = rootCmt;
  replyTarget.value = rootCmt;
  floorPage.value = 1;
  floorHasMore.value = true;
  floorReplies.value = [];
  if (!targetSong.value) return;
  isLoadingFloor.value = true;
  try {
    const list = await kugouClient.getCommentFloorReplies(
      targetSong.value,
      rootCmt.id,
      rootCmt.specialChildId,
      1
    );
    floorReplies.value = enrichFloorTreeLevels(rootCmt, list);
    floorHasMore.value = list.length >= 15;
  } catch {
    floorReplies.value = [];
    floorHasMore.value = false;
  } finally {
    isLoadingFloor.value = false;
  }
}

async function loadMoreFloorReplies() {
  if (!targetSong.value || !activeFloorRoot.value || isLoadingFloor.value || !floorHasMore.value)
    return;
  isLoadingFloor.value = true;
  try {
    const nextP = floorPage.value + 1;
    const more = await kugouClient.getCommentFloorReplies(
      targetSong.value,
      activeFloorRoot.value.id,
      activeFloorRoot.value.specialChildId,
      nextP
    );
    if (more.length === 0) {
      floorHasMore.value = false;
    } else {
      const seen = new Set(floorReplies.value.map((r) => r.id));
      const unique = more.filter((r) => !seen.has(r.id));
      const combined = [...floorReplies.value, ...unique];
      floorReplies.value = enrichFloorTreeLevels(activeFloorRoot.value, combined);
      floorPage.value = nextP;
      if (more.length < 15 || unique.length === 0) {
        floorHasMore.value = false;
      }
    }
  } catch {
    floorHasMore.value = false;
  } finally {
    isLoadingFloor.value = false;
  }
}

interface EnrichedReply extends SongComment {
  nestLevel: number;
  parentComment?: SongComment;
}

function enrichFloorTreeLevels(root: SongComment, replies: SongComment[]): EnrichedReply[] {
  const byId = new Map<string, SongComment>();
  byId.set(root.id, root);
  for (const r of replies) {
    byId.set(r.id, r);
  }
  const byUser = new Map<string, SongComment>();
  byUser.set(root.userName, root);
  for (const r of replies) {
    if (r.userName) byUser.set(r.userName, r);
  }

  return replies.map((r) => {
    let level = 1;
    let parent: SongComment | undefined = undefined;
    if (r.replyTargetCommentId && byId.has(r.replyTargetCommentId)) {
      parent = byId.get(r.replyTargetCommentId);
    } else if (r.replyToUser && byUser.has(r.replyToUser)) {
      parent = byUser.get(r.replyToUser);
    }
    if (r.replyChainHops && r.replyChainHops.length > 0) {
      level = Math.min(7, r.replyChainHops.length + 1);
    } else if (parent && parent.id !== root.id) {
      level = 2;
    }
    return {
      ...r,
      nestLevel: Math.max(1, Math.min(7, level)),
      parentComment: parent && parent.id !== root.id ? parent : undefined,
    };
  });
}

function openConversationModal(reply: EnrichedReply) {
  if (!activeFloorRoot.value) return;
  const chain: SongComment[] = [activeFloorRoot.value];
  if (reply.parentComment && reply.parentComment.id !== activeFloorRoot.value.id) {
    chain.push(reply.parentComment);
  }
  if (reply.replyChainHops && reply.replyChainHops.length > 0) {
    for (let i = reply.replyChainHops.length - 1; i >= 0; i--) {
      const hop = reply.replyChainHops[i];
      chain.push({
        id: `hop_${i}`,
        userName: hop.userName,
        userAvatar: "",
        content: hop.content,
        createdTime: "历史引用",
        likeCount: 0,
        replyCount: 0,
        timeText: "历史引用",
        isLiked: false,
        location: "",
        images: [],
        specialChildId: "",
        replies: [],
      });
    }
  }
  chain.push(reply);
  conversationChain.value = chain;
}

function selectReplyTarget(cmt: SongComment) {
  replyTarget.value = cmt;
}

function insertEmoji(em: string) {
  commentInput.value += em;
}

async function handleSendComment() {
  const text = commentInput.value.trim();
  if (!text || isSubmitting.value) return;
  isSubmitting.value = true;
  try {
    const targetId = replyTarget.value?.id || activeFloorRoot.value?.id;
    const ok = await store.submitSongComment(text, targetId);
    if (ok) {
      if (activeFloorRoot.value) {
        floorReplies.value.unshift({
          id: `local_${Date.now()}`,
          userName: store.profile?.nickname || store.session?.nickname || "我",
          userAvatar: store.profile?.avatarUrl || store.session?.avatarUrl || "",
          content: text,
          createdTime: "刚刚",
          likeCount: 0,
          replyCount: 0,
          timeText: "刚刚",
          isLiked: false,
          location: "",
          images: [],
          specialChildId: "",
          replies: [],
          replyToUser: replyTarget.value?.userName,
          nestLevel: replyTarget.value && replyTarget.value.id !== activeFloorRoot.value.id ? 2 : 1,
        });
        activeFloorRoot.value.replyCount += 1;
      }
      commentInput.value = "";
      showEmojiPicker.value = false;
    }
  } finally {
    isSubmitting.value = false;
  }
}

// 将根评论的热门前 3 条内联子回复提取展示（带 SVG 弧形分支线，对齐 Android CommentsScreen.kt）
function getInlinePreviewReplies(cmt: SongComment): Array<{ userName: string; content: string }> {
  const cached = store.floorRepliesMap[cmt.id];
  if (cached && cached.length > 0) {
    return cached.slice(0, 3).map((r) => ({
      userName: r.userName,
      content: r.content,
    }));
  }
  if (cmt.replyChainHops && cmt.replyChainHops.length > 0) {
    return cmt.replyChainHops.slice(0, 3);
  }
  return [];
}
</script>

<template>
  <transition name="fade">
    <div
      v-if="store.isCommentsModalOpen"
      class="modal-backdrop comments-modal-backdrop"
      @click.self="store.closeCommentsModal()"
    >
      <div class="comments-modal-shell">
        <!-- 顶部歌曲上下文胶囊条 + 热门/最新分段开关 -->
        <div class="cm-header">
          <div class="cm-header-left">
            <button
              v-if="activeFloorRoot"
              class="cm-back-floor-btn"
              title="返回主评论列表"
              @click="activeFloorRoot = null; replyTarget = null"
            >
              <PlumpIcon name="arrow-left" :size="16" />
              <span>返回热评</span>
            </button>

            <div v-if="targetSong" class="cm-song-pill">
              <img
                v-if="targetSong.coverUrl"
                :src="targetSong.coverUrl"
                class="cm-song-cover"
                alt="cover"
              />
              <div class="cm-song-meta">
                <div class="cm-song-title">
                  <span>{{ targetSong.title }}</span>
                  <span v-if="targetSong.isVip" class="mini-badge vip">VIP</span>
                  <span v-if="targetSong.isHiRes" class="mini-badge hires">Hi-Res</span>
                  <span v-else-if="targetSong.isSq" class="mini-badge sq">SQ</span>
                </div>
                <div class="cm-song-sub">
                  <button
                    class="cm-link-inline"
                    @click="store.closeCommentsModal(); store.navigateToSongArtist(targetSong)"
                  >
                    {{ targetSong.artist }}
                  </button>
                  <template v-if="targetSong.album">
                    <span> · </span>
                    <button
                      class="cm-link-inline"
                      @click="store.closeCommentsModal(); store.navigateToSongAlbum(targetSong)"
                    >
                      {{ targetSong.album }}
                    </button>
                  </template>
                </div>
              </div>
            </div>
          </div>

          <div class="cm-header-right">
            <div v-if="!activeFloorRoot" class="cm-sort-capsule">
              <button
                class="cm-sort-btn"
                :class="{ active: store.commentSortType === 'hot' }"
                @click="switchSort('hot')"
              >
                🔥 最热评论
              </button>
              <button
                class="cm-sort-btn"
                :class="{ active: store.commentSortType === 'new' }"
                @click="switchSort('new')"
              >
                🕒 最新流光
              </button>
            </div>
            <span v-else class="cm-floor-badge">
              楼中楼详情 · {{ activeFloorRoot.replyCount || floorReplies.length }} 条回复
            </span>

            <button
              class="icon-close-btn"
              title="关闭评论弹窗"
              @click="store.closeCommentsModal()"
            >
              <PlumpIcon name="close" :size="16" />
            </button>
          </div>
        </div>

        <!-- 主体滚动区域：1. 根评论流 或 2. 楼中楼树状详情页 -->
        <div class="cm-body-scroll">
          <!-- A. 楼中楼树状详情页（复刻 Android FloorCommentsSheet 1~7 级炫彩树状指示条） -->
          <template v-if="activeFloorRoot">
            <div class="cm-floor-owner-card">
              <div class="cm-card-top">
                <img
                  v-if="activeFloorRoot.userAvatar"
                  :src="activeFloorRoot.userAvatar"
                  class="cm-avatar lg"
                  alt="avatar"
                />
                <div v-else class="cm-avatar-fallback lg">
                  {{ activeFloorRoot.userName.slice(0, 1) }}
                </div>
                <div class="cm-user-col">
                  <div class="cm-user-row">
                    <span class="cm-username">{{ activeFloorRoot.userName }}</span>
                    <span class="cm-owner-tag">楼主</span>
                    <span v-if="activeFloorRoot.vipLevel && activeFloorRoot.vipLevel > 0" class="cm-vip-tag">
                      VIP {{ activeFloorRoot.vipLevel }}
                    </span>
                  </div>
                  <div class="cm-time-loc">
                    <span>{{ activeFloorRoot.timeText }}</span>
                    <span v-if="activeFloorRoot.location"> · IP {{ activeFloorRoot.location }}</span>
                  </div>
                </div>
                <button
                  class="cm-like-btn"
                  :class="{ liked: activeFloorRoot.isLiked }"
                  @click="store.toggleLikeComment(activeFloorRoot)"
                >
                  <PlumpIcon name="heart" :size="15" />
                  <span>{{ activeFloorRoot.likeCount || 0 }}</span>
                </button>
              </div>

              <div class="cm-content-text lg">{{ activeFloorRoot.content }}</div>

              <div v-if="activeFloorRoot.images && activeFloorRoot.images.length > 0" class="cm-img-grid">
                <img
                  v-for="(imgUrl, idx) in activeFloorRoot.images"
                  :key="idx"
                  :src="imgUrl"
                  class="cm-comment-img"
                  alt="comment"
                  @click="previewImageUrl = imgUrl"
                />
              </div>
            </div>

            <div class="cm-floor-divider">
              <span>全部楼层回复 ({{ floorReplies.length }})</span>
            </div>

            <div v-if="isLoadingFloor && floorReplies.length === 0" class="cm-empty-state">
              正在加载楼中楼树状回复...
            </div>
            <div v-else-if="floorReplies.length === 0" class="cm-empty-state">
              暂无子楼层回复，快来抢沙发吧～
            </div>

            <div v-else class="cm-floor-tree-list">
              <div
                v-for="rep in (floorReplies as EnrichedReply[])"
                :key="rep.id"
                class="cm-floor-reply-item"
                :style="{
                  paddingLeft: `${14 + (rep.nestLevel - 1) * 16}px`,
                }"
              >
                <!-- 左侧 1~7 级炫彩树状导轨线 -->
                <div class="cm-tree-guides">
                  <span
                    v-for="lv in rep.nestLevel"
                    :key="lv"
                    class="cm-tree-guide-bar"
                    :style="{
                      left: `${(lv - 1) * 16 + 4}px`,
                      background: getClusterColor(lv),
                    }"
                  ></span>
                </div>

                <div class="cm-floor-reply-card" @click="selectReplyTarget(rep)">
                  <div class="cm-card-top">
                    <img
                      v-if="rep.userAvatar"
                      :src="rep.userAvatar"
                      class="cm-avatar sm"
                      alt="avatar"
                    />
                    <div v-else class="cm-avatar-fallback sm">
                      {{ rep.userName.slice(0, 1) }}
                    </div>
                    <div class="cm-user-col">
                      <div class="cm-user-row">
                        <span class="cm-username">{{ rep.userName }}</span>
                        <span
                          v-if="rep.userName === activeFloorRoot.userName"
                          class="cm-owner-tag"
                        >
                          楼主
                        </span>
                        <span
                          class="cm-level-pill"
                          :style="{
                            color: getClusterColor(rep.nestLevel),
                            borderColor: getClusterColor(rep.nestLevel) + '55',
                          }"
                        >
                          L{{ rep.nestLevel }}
                        </span>
                        <span v-if="rep.replyToUser" class="cm-reply-to-label">
                          回复 <strong>@{{ rep.replyToUser }}</strong>
                        </span>
                      </div>
                      <div class="cm-time-loc">
                        <span>{{ rep.timeText }}</span>
                        <span v-if="rep.location"> · {{ rep.location }}</span>
                      </div>
                    </div>

                    <div class="cm-reply-actions">
                      <button
                        v-if="rep.parentComment || (rep.replyChainHops && rep.replyChainHops.length > 0)"
                        class="cm-conv-btn"
                        @click.stop="openConversationModal(rep)"
                      >
                        查看对话
                      </button>
                      <button
                        class="cm-like-btn"
                        :class="{ liked: rep.isLiked }"
                        @click.stop="store.toggleLikeComment(rep)"
                      >
                        <PlumpIcon name="heart" :size="14" />
                        <span>{{ rep.likeCount || 0 }}</span>
                      </button>
                    </div>
                  </div>

                  <!-- 若存在被回复的上级评论预览框 -->
                  <div v-if="rep.parentComment" class="cm-quoted-parent-box">
                    <span class="cm-qp-user">@{{ rep.parentComment.userName }}：</span>
                    <span class="cm-qp-text">{{ rep.parentComment.content }}</span>
                  </div>

                  <div class="cm-content-text">{{ rep.content }}</div>
                </div>
              </div>

              <div v-if="floorHasMore" class="cm-load-more-row">
                <button
                  class="tonal-pill-btn"
                  :disabled="isLoadingFloor"
                  @click="loadMoreFloorReplies"
                >
                  {{ isLoadingFloor ? "加载中..." : "加载更多楼中楼回复" }}
                </button>
              </div>
            </div>
          </template>

          <!-- B. 主评论列表（复刻 Android CommentsScreen：热评徽章 + 3 条内联回复带弧形分支线） -->
          <template v-else>
            <div v-if="store.isLoadingComments && store.activeSongComments.length === 0" class="cm-empty-state">
              正在从酷狗概念版云端同步乐评...
            </div>
            <div v-else-if="store.activeSongComments.length === 0" class="cm-empty-state">
              这首歌还没有人留下足迹，快来写下第一条乐评吧 ✨
            </div>

            <div v-else class="cm-main-list">
              <div
                v-for="cmt in store.activeSongComments"
                :key="cmt.id"
                class="cm-root-card"
              >
                <div class="cm-card-top">
                  <img
                    v-if="cmt.userAvatar"
                    :src="cmt.userAvatar"
                    class="cm-avatar"
                    alt="avatar"
                  />
                  <div v-else class="cm-avatar-fallback">
                    {{ cmt.userName.slice(0, 1) }}
                  </div>

                  <div class="cm-user-col">
                    <div class="cm-user-row">
                      <span class="cm-username">{{ cmt.userName }}</span>
                      <span v-if="cmt.isHot || cmt.likeCount >= 100" class="cm-hot-badge">
                        HOT 热评
                      </span>
                      <span v-if="cmt.vipLevel && cmt.vipLevel > 0" class="cm-vip-tag">
                        VIP {{ cmt.vipLevel }}
                      </span>
                    </div>
                    <div class="cm-time-loc">
                      <span>{{ cmt.timeText }}</span>
                      <span v-if="cmt.location"> · IP {{ cmt.location }}</span>
                    </div>
                  </div>

                  <div class="cm-reply-actions">
                    <button
                      class="cm-action-pill"
                      title="回复该乐评"
                      @click="selectReplyTarget(cmt)"
                    >
                      回复
                    </button>
                    <button
                      class="cm-like-btn"
                      :class="{ liked: cmt.isLiked }"
                      @click="store.toggleLikeComment(cmt)"
                    >
                      <PlumpIcon name="heart" :size="15" />
                      <span>{{ cmt.likeCount || 0 }}</span>
                    </button>
                  </div>
                </div>

                <div class="cm-content-text" @click="selectReplyTarget(cmt)">
                  {{ cmt.content }}
                </div>

                <!-- 评论配图网格 -->
                <div v-if="cmt.images && cmt.images.length > 0" class="cm-img-grid">
                  <img
                    v-for="(imgUrl, idx) in cmt.images"
                    :key="idx"
                    :src="imgUrl"
                    class="cm-comment-img"
                    alt="comment"
                    @click="previewImageUrl = imgUrl"
                  />
                </div>

                <!-- 内联子回复预览区（带贝塞尔弧形分支连接线 + 查看全部 X 条回复入口） -->
                <div
                  v-if="cmt.replyCount > 0 || getInlinePreviewReplies(cmt).length > 0"
                  class="cm-inline-replies-box"
                  @click="openFloorDetail(cmt)"
                >
                  <svg class="cm-bezier-branch-svg" width="22" height="100%" viewBox="0 0 22 60" preserveAspectRatio="none">
                    <path
                      d="M 6 0 L 6 38 Q 6 50 18 50"
                      fill="none"
                      stroke="rgba(56, 189, 248, 0.45)"
                      stroke-width="1.8"
                      stroke-linecap="round"
                    />
                  </svg>

                  <div class="cm-inline-replies-content">
                    <div
                      v-for="(sub, sIdx) in getInlinePreviewReplies(cmt)"
                      :key="sIdx"
                      class="cm-inline-reply-line"
                    >
                      <span class="cm-ir-user">{{ sub.userName }}：</span>
                      <span class="cm-ir-text">{{ sub.content }}</span>
                    </div>

                    <div class="cm-open-floor-link">
                      <span>
                        共 {{ Math.max(cmt.replyCount, getInlinePreviewReplies(cmt).length) }} 条回复 · 点击展开楼中楼详情
                      </span>
                      <PlumpIcon name="chevron-right" :size="14" />
                    </div>
                  </div>
                </div>
              </div>

              <div v-if="mainHasMore" class="cm-load-more-row">
                <button
                  class="tonal-pill-btn"
                  :disabled="isLoadingMoreMain"
                  @click="loadMoreMainComments"
                >
                  {{ isLoadingMoreMain ? "正在加载更多评论..." : "加载更多乐评" }}
                </button>
              </div>
            </div>
          </template>
        </div>

        <!-- 底部评论发送与表情栏 -->
        <div class="cm-composer-footer">
          <div v-if="replyTarget" class="cm-replying-banner">
            <span>正在回复 <strong>@{{ replyTarget.userName }}</strong>：{{ replyTarget.content.slice(0, 32) }}</span>
            <button class="cm-cancel-reply" @click="replyTarget = activeFloorRoot || null">
              取消指定
            </button>
          </div>

          <div v-if="showEmojiPicker" class="cm-emoji-bar">
            <button
              v-for="em in emojiList"
              :key="em"
              class="cm-emoji-btn"
              @click="insertEmoji(em)"
            >
              {{ em }}
            </button>
          </div>

          <div class="cm-input-row">
            <button
              class="cm-emoji-trigger"
              :class="{ active: showEmojiPicker }"
              title="插入表情"
              @click="showEmojiPicker = !showEmojiPicker"
            >
              😊
            </button>
            <input
              v-model="commentInput"
              type="text"
              class="cm-text-input"
              :placeholder="
                replyTarget
                  ? `回复 @${replyTarget.userName}...`
                  : '分享你此刻听歌的感受与故事...'
              "
              @keydown.enter="handleSendComment"
            />
            <button
              class="primary-btn cm-send-btn"
              :disabled="!commentInput.trim() || isSubmitting"
              @click="handleSendComment"
            >
              <span>{{ isSubmitting ? "发送中..." : "发送评论" }}</span>
            </button>
          </div>
        </div>

        <!-- 查看对话上下文子弹窗 -->
        <transition name="fade">
          <div
            v-if="conversationChain"
            class="cm-submodal-backdrop"
            @click.self="conversationChain = null"
          >
            <div class="cm-conversation-dialog">
              <div class="cm-conv-head">
                <h4>对话上下文链路 ({{ conversationChain.length }} 层)</h4>
                <button class="icon-close-btn" @click="conversationChain = null">
                  <PlumpIcon name="close" :size="14" />
                </button>
              </div>
              <div class="cm-conv-list">
                <div
                  v-for="(node, idx) in conversationChain"
                  :key="node.id + '_' + idx"
                  class="cm-conv-step"
                >
                  <div class="cm-conv-step-badge">{{ idx + 1 }}</div>
                  <div class="cm-conv-step-body">
                    <div class="cm-conv-user">
                      <strong>{{ node.userName }}</strong>
                      <span>{{ node.timeText }}</span>
                    </div>
                    <div class="cm-conv-text">{{ node.content }}</div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </transition>

        <!-- 配图大图预览 -->
        <transition name="fade">
          <div
            v-if="previewImageUrl"
            class="cm-submodal-backdrop"
            @click="previewImageUrl = null"
          >
            <img :src="previewImageUrl" class="cm-preview-large-img" alt="preview" />
          </div>
        </transition>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.comments-modal-backdrop {
  z-index: 2600;
  backdrop-filter: blur(18px);
}
.comments-modal-shell {
  width: 780px;
  max-width: 94vw;
  height: 82vh;
  max-height: 820px;
  border-radius: 24px;
  background: linear-gradient(165deg, rgba(20, 26, 38, 0.96), rgba(12, 16, 26, 0.98));
  border: 1px solid rgba(255, 255, 255, 0.12);
  box-shadow: 0 28px 80px rgba(0, 0, 0, 0.65);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  position: relative;
}
.cm-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 16px 22px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.02);
}
.cm-header-left {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}
.cm-back-floor-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 999px;
  background: rgba(56, 189, 248, 0.14);
  border: 1px solid rgba(56, 189, 248, 0.35);
  color: #38bdf8;
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
}
.cm-song-pill {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}
.cm-song-cover {
  width: 42px;
  height: 42px;
  border-radius: 10px;
  object-fit: cover;
  flex-shrink: 0;
}
.cm-song-meta {
  min-width: 0;
}
.cm-song-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 15px;
  font-weight: 800;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.cm-song-sub {
  font-size: 12.5px;
  color: var(--text-secondary);
  margin-top: 2px;
}
.cm-link-inline {
  background: none;
  border: none;
  padding: 0;
  color: var(--cyan);
  font-size: 12.5px;
  cursor: pointer;
}
.cm-link-inline:hover {
  text-decoration: underline;
}
.cm-header-right {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}
.cm-sort-capsule {
  display: flex;
  background: rgba(255, 255, 255, 0.06);
  padding: 3px;
  border-radius: 999px;
}
.cm-sort-btn {
  border: none;
  background: transparent;
  color: var(--text-secondary);
  padding: 5px 13px;
  border-radius: 999px;
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.2s ease;
}
.cm-sort-btn.active {
  background: var(--cyan);
  color: #06141f;
}
.cm-floor-badge {
  font-size: 13px;
  font-weight: 700;
  color: #38bdf8;
  background: rgba(56, 189, 248, 0.12);
  padding: 5px 12px;
  border-radius: 999px;
}
.cm-body-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 18px 22px;
}
.cm-empty-state {
  text-align: center;
  padding: 64px 20px;
  color: var(--text-secondary);
  font-size: 14px;
}
.cm-main-list {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.cm-root-card,
.cm-floor-owner-card {
  padding: 16px 18px;
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid rgba(255, 255, 255, 0.07);
  transition: border-color 0.2s ease;
}
.cm-root-card:hover {
  border-color: rgba(255, 255, 255, 0.16);
}
.cm-card-top {
  display: flex;
  align-items: center;
  gap: 12px;
}
.cm-avatar,
.cm-avatar-fallback {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  object-fit: cover;
  flex-shrink: 0;
}
.cm-avatar.sm,
.cm-avatar-fallback.sm {
  width: 30px;
  height: 30px;
}
.cm-avatar.lg,
.cm-avatar-fallback.lg {
  width: 44px;
  height: 44px;
}
.cm-avatar-fallback {
  background: linear-gradient(135deg, #38bdf8, #6366f1);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 800;
  color: #fff;
  font-size: 14px;
}
.cm-user-col {
  flex: 1;
  min-width: 0;
}
.cm-user-row {
  display: flex;
  align-items: center;
  gap: 7px;
  flex-wrap: wrap;
}
.cm-username {
  font-weight: 700;
  font-size: 14px;
  color: var(--text-primary);
}
.cm-hot-badge {
  font-size: 10.5px;
  font-weight: 800;
  padding: 1px 7px;
  border-radius: 999px;
  background: rgba(244, 63, 94, 0.18);
  color: #fb7185;
  border: 1px solid rgba(244, 63, 94, 0.35);
}
.cm-owner-tag {
  font-size: 10.5px;
  font-weight: 800;
  padding: 1px 7px;
  border-radius: 999px;
  background: rgba(56, 189, 248, 0.18);
  color: #38bdf8;
}
.cm-vip-tag {
  font-size: 10.5px;
  font-weight: 800;
  padding: 1px 6px;
  border-radius: 6px;
  background: rgba(245, 158, 11, 0.18);
  color: #fbbf24;
}
.cm-level-pill {
  font-size: 10.5px;
  font-weight: 800;
  padding: 0 6px;
  border-radius: 6px;
  border: 1px solid;
}
.cm-reply-to-label {
  font-size: 12px;
  color: var(--text-secondary);
}
.cm-time-loc {
  font-size: 11.5px;
  color: var(--text-secondary);
  margin-top: 2px;
}
.cm-reply-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.cm-action-pill,
.cm-conv-btn {
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: var(--text-secondary);
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 12px;
  cursor: pointer;
}
.cm-action-pill:hover,
.cm-conv-btn:hover {
  color: var(--cyan);
  border-color: rgba(0, 229, 255, 0.4);
}
.cm-like-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: var(--text-secondary);
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 12px;
  cursor: pointer;
}
.cm-like-btn.liked {
  color: #fb7185;
  border-color: rgba(251, 113, 133, 0.4);
  background: rgba(251, 113, 133, 0.12);
}
.cm-content-text {
  margin-top: 10px;
  font-size: 14.5px;
  line-height: 1.62;
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
  cursor: pointer;
}
.cm-content-text.lg {
  font-size: 15.5px;
}
.cm-img-grid {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 10px;
}
.cm-comment-img {
  width: 92px;
  height: 92px;
  border-radius: 10px;
  object-fit: cover;
  cursor: zoom-in;
  border: 1px solid rgba(255, 255, 255, 0.1);
}
.cm-inline-replies-box {
  margin-top: 12px;
  padding: 10px 14px 10px 8px;
  border-radius: 14px;
  background: rgba(0, 0, 0, 0.24);
  border: 1px solid rgba(56, 189, 248, 0.16);
  display: flex;
  align-items: stretch;
  gap: 6px;
  cursor: pointer;
  transition: background 0.2s ease;
}
.cm-inline-replies-box:hover {
  background: rgba(56, 189, 248, 0.08);
}
.cm-bezier-branch-svg {
  flex-shrink: 0;
}
.cm-inline-replies-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.cm-inline-reply-line {
  font-size: 13px;
  line-height: 1.45;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cm-ir-user {
  color: #38bdf8;
  font-weight: 700;
}
.cm-open-floor-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12.5px;
  font-weight: 700;
  color: var(--cyan);
  margin-top: 2px;
}
.cm-floor-divider {
  margin: 16px 0 12px;
  font-size: 13px;
  font-weight: 800;
  color: var(--text-secondary);
}
.cm-floor-tree-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.cm-floor-reply-item {
  position: relative;
}
.cm-tree-guides {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 0;
  pointer-events: none;
}
.cm-tree-guide-bar {
  position: absolute;
  top: 4px;
  bottom: 4px;
  width: 2.5px;
  border-radius: 999px;
  opacity: 0.75;
}
.cm-floor-reply-card {
  padding: 12px 14px;
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.06);
  cursor: pointer;
}
.cm-floor-reply-card:hover {
  background: rgba(255, 255, 255, 0.05);
}
.cm-quoted-parent-box {
  margin-top: 8px;
  padding: 7px 10px;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.04);
  border-left: 3px solid #38bdf8;
  font-size: 12.5px;
  color: var(--text-secondary);
}
.cm-qp-user {
  color: #38bdf8;
  font-weight: 700;
}
.cm-load-more-row {
  display: flex;
  justify-content: center;
  padding: 14px 0 6px;
}
.cm-composer-footer {
  padding: 12px 20px 16px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(10, 14, 24, 0.95);
}
.cm-replying-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  color: #38bdf8;
  background: rgba(56, 189, 248, 0.1);
  padding: 5px 12px;
  border-radius: 8px;
  margin-bottom: 8px;
}
.cm-cancel-reply {
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 12px;
}
.cm-emoji-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 8px 10px;
  margin-bottom: 8px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.05);
}
.cm-emoji-btn {
  background: none;
  border: none;
  font-size: 18px;
  padding: 3px 5px;
  cursor: pointer;
  border-radius: 6px;
}
.cm-emoji-btn:hover {
  background: rgba(255, 255, 255, 0.12);
}
.cm-input-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.cm-emoji-trigger {
  width: 38px;
  height: 38px;
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.05);
  font-size: 18px;
  cursor: pointer;
}
.cm-text-input {
  flex: 1;
  height: 38px;
  padding: 0 14px;
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(255, 255, 255, 0.05);
  color: var(--text-primary);
  font-size: 13.5px;
}
.cm-send-btn {
  height: 38px;
  padding: 0 18px;
}
.cm-submodal-backdrop {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.72);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 20;
}
.cm-conversation-dialog {
  width: 520px;
  max-width: 90%;
  max-height: 75%;
  background: #141a29;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 18px;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.cm-conv-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.cm-conv-list {
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.cm-conv-step {
  display: flex;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.04);
}
.cm-conv-step-badge {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--cyan);
  color: #05131d;
  font-size: 12px;
  font-weight: 800;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.cm-conv-user {
  display: flex;
  justify-content: space-between;
  font-size: 12.5px;
  margin-bottom: 4px;
}
.cm-conv-text {
  font-size: 13.5px;
  line-height: 1.5;
}
.cm-preview-large-img {
  max-width: 88%;
  max-height: 84%;
  border-radius: 14px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.8);
}
</style>
