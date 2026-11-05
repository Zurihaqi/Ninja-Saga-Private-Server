"""Optional event banners must not hold the login pipeline indefinitely."""

def patch(source):
    if 'onBannerTimeout' in source:
        return source
    source=source.replace('   import Storage.Character;','   import Storage.Character;\n   import flash.display.Bitmap;\n   import flash.display.BitmapData;\n   import flash.utils.Timer;\n   import flash.events.TimerEvent;')
    source=source.replace('      private var loaded:* = 0;','      private var loaded:* = 0;\n      private var completed:Boolean = false;\n      private var bannerTimer:Timer;')
    source=source.replace('            this.data = param1.events;\n            this.loadImages();','''            this.data = param1.events;
            if(this.data && this.data.seasonal)
            {
               for each(var banner:Object in this.data.seasonal)
               {
                  banner.image = new Bitmap(new BitmapData(320,160,false,0x253445));
               }
            }
            this.bannerTimer = new Timer(5000,1);
            this.bannerTimer.addEventListener(TimerEvent.TIMER_COMPLETE,this.onBannerTimeout);
            this.bannerTimer.start();
            this.loadImages();''')
    for signature in ['internal function completeImg(param1:*) : *','internal function errImg(param1:*) : *','internal function checkComplete() : *','internal function onImageComplete(param1:*) : void','internal function onImageError(param1:ErrorEvent) : void','private function onLoadError(param1:ErrorEvent) : *']:
        old='      '+signature+'\n      {'
        if old not in source:raise ValueError('Banner handler contract changed: '+signature)
        source=source.replace(old,old+'\n         if(this.completed) return;',1)
    start=source.index('      public function complete() : *')
    end=source.index('      public function destroy() : *',start)
    source=source[:start]+'''      private function onBannerTimeout(event:TimerEvent) : void
      {
         this.complete();
      }

      public function complete() : *
      {
         if(this.completed) return;
         this.completed = true;
         if(this.bannerTimer)
         {
            this.bannerTimer.stop();
            this.bannerTimer.removeEventListener(TimerEvent.TIMER_COMPLETE,this.onBannerTimeout);
            this.bannerTimer = null;
         }
         Character.event_data = this.data ? this.data : {"seasonal":[],"event:permanent":[],"features":[],"packages":null};
         if(this.splash) this.splash.reset();
         dispatchEvent(new TaskEvent(TaskEvent.COMPLETE));
         this.destroy();
      }

'''+source[end:]
    source=source.replace('         this.tempLoader.removeEventListener(BulkLoader.COMPLETE,this.onImageComplete);\n         this.tempLoader.removeEventListener(BulkLoader.ERROR,this.onImageError);\n         this.tempLoader.clear();','''         if(this.tempLoader)
         {
            this.tempLoader.removeEventListener(BulkLoader.COMPLETE,this.onImageComplete);
            this.tempLoader.removeEventListener(BulkLoader.ERROR,this.onImageError);
            this.tempLoader.removeAll();
         }''')
    return source
PATCHES={'id.ninjasage.tasks.core.ImageDownloadTask':patch}
VERIFICATIONS={'id.ninjasage.tasks.core.ImageDownloadTask':['onBannerTimeout','this.completed = true','5000','Character.event_data']}
